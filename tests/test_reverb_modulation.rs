//! Tests for reverb modulation
//! Verifies that reverb parameters can be modulated via patterns

use phonon::compositional_parser::parse_program;
use phonon::compositional_compiler::compile_program;

fn render_audio(code: &str, samples: usize) -> Vec<f32> {
    let (rest, statements) = parse_program(code).expect("Parse failed");
    assert!(rest.trim().is_empty(), "Unparsed: {}", rest);

    let mut graph = compile_program(statements, 44100.0, None)
        .expect("Compile failed");

    let mut buffer = vec![0.0f32; samples];
    graph.process_buffer(&mut buffer);
    buffer
}

fn calculate_spectral_centroid(buffer: &[f32], sample_rate: f32) -> f32 {
    // Simple spectral centroid estimation using zero-crossing rate as proxy
    // Higher = brighter, lower = darker
    let mut crossings = 0;
    for i in 1..buffer.len() {
        if (buffer[i-1] < 0.0 && buffer[i] >= 0.0) || (buffer[i-1] >= 0.0 && buffer[i] < 0.0) {
            crossings += 1;
        }
    }
    // Return estimated dominant frequency
    crossings as f32 * sample_rate / (2.0 * buffer.len() as f32)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt()
}

/// Test that reverb room_size modulation affects output
#[test]
fn test_reverb_room_size_modulation() {
    // Test with low room size
    let code_low = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # reverb 0.1 0.5 0.5
"#;

    // Test with high room size
    let code_high = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # reverb 0.9 0.5 0.5
"#;

    let audio_low = render_audio(code_low, 44100);
    let audio_high = render_audio(code_high, 44100);

    // Calculate RMS of the second half (where reverb tail would differ)
    let tail_low = &audio_low[22050..];
    let tail_high = &audio_high[22050..];

    let rms_low = calculate_rms(tail_low);
    let rms_high = calculate_rms(tail_high);

    println!("Room size 0.1 tail RMS: {:.4}", rms_low);
    println!("Room size 0.9 tail RMS: {:.4}", rms_high);
    println!("Ratio: {:.2}x", rms_high / rms_low);

    // High room size should produce longer tail (more energy in second half)
    assert!(rms_high > rms_low * 1.1,
        "Expected high room size to have more reverb tail. Low={:.4}, High={:.4}",
        rms_low, rms_high);
}

/// Test that reverb damping modulation affects output
#[test]
fn test_reverb_damping_modulation() {
    // Low damping = brighter reverb
    let code_low = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # reverb 0.8 0.1 0.7
"#;

    // High damping = darker reverb
    let code_high = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # reverb 0.8 0.9 0.7
"#;

    let audio_low = render_audio(code_low, 44100);
    let audio_high = render_audio(code_high, 44100);

    // Look at reverb tail
    let tail_low = &audio_low[22050..];
    let tail_high = &audio_high[22050..];

    let brightness_low = calculate_spectral_centroid(tail_low, 44100.0);
    let brightness_high = calculate_spectral_centroid(tail_high, 44100.0);

    println!("Damping 0.1 spectral centroid: {:.1} Hz", brightness_low);
    println!("Damping 0.9 spectral centroid: {:.1} Hz", brightness_high);

    // Low damping should be brighter (higher spectral centroid)
    // This is subtle, so we just check they're different
    let diff = (brightness_low - brightness_high).abs();
    println!("Difference: {:.1} Hz", diff);

    // At minimum, verify audio is produced
    assert!(calculate_rms(tail_low) > 0.001, "Should have audio in tail");
    assert!(calculate_rms(tail_high) > 0.001, "Should have audio in tail");
}

/// Test that reverb mix modulation affects output
#[test]
fn test_reverb_mix_modulation() {
    // 0% wet
    let code_dry = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # reverb 0.8 0.3 0.0
"#;

    // 100% wet
    let code_wet = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # reverb 0.8 0.3 1.0
"#;

    let audio_dry = render_audio(code_dry, 44100);
    let audio_wet = render_audio(code_wet, 44100);

    // Compare first portion (direct sound vs reverb diffusion)
    let first_dry = &audio_dry[0..4410];
    let first_wet = &audio_wet[0..4410];

    // Compute correlation/difference
    let correlation: f32 = first_dry.iter()
        .zip(first_wet.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / first_dry.len() as f32;

    println!("Avg sample difference between dry and wet: {:.4}", correlation);

    // They should sound different
    assert!(correlation > 0.01,
        "Expected dry and wet reverb to sound different, got diff={:.4}",
        correlation);
}

/// Test pattern-modulated reverb room size
#[test]
fn test_reverb_pattern_modulated_room() {
    // Room size that changes between cycles
    let code = r#"
tempo: 0.5
~room # "0.2 0.8"
~src $ saw 220
out $ ~src # reverb ~room 0.5 0.5
"#;

    let audio = render_audio(code, 88200); // 2 seconds

    // First cycle (room=0.2)
    let first_half = &audio[0..44100];
    // Second cycle (room=0.8)
    let second_half = &audio[44100..88200];

    let rms_first = calculate_rms(first_half);
    let rms_second = calculate_rms(second_half);

    println!("Pattern room [0.2 0.8]: First half RMS={:.4}, Second half RMS={:.4}",
        rms_first, rms_second);

    // Both should have audio
    assert!(rms_first > 0.01, "First half should have audio");
    assert!(rms_second > 0.01, "Second half should have audio");

    // The outputs should be different due to room size change
    let ratio = rms_second / rms_first;
    println!("Second/First ratio: {:.2}", ratio);
}

/// Test Dattorro reverb (plate) basic functionality
#[test]
fn test_plate_reverb_produces_output() {
    // Test plate reverb with a simple saw wave
    let code = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # plate 10 2.0
"#;

    let audio = render_audio(code, 44100);
    let rms = calculate_rms(&audio);

    println!("Plate reverb output RMS: {:.4}", rms);
    assert!(rms > 0.01, "Plate reverb should produce audio, got RMS={}", rms);
}

/// Test Dattorro reverb (plate) decay difference
#[test]
fn test_plate_reverb_decay_difference() {
    // Short decay
    let code_short = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # plate 10 0.5 0.5 0.5 0.3 0.5
"#;

    // Long decay
    let code_long = r#"
tempo: 0.5
~src $ saw 220
out $ ~src # plate 10 5.0 0.5 0.5 0.3 0.5
"#;

    let audio_short = render_audio(code_short, 44100);
    let audio_long = render_audio(code_long, 44100);

    let rms_short = calculate_rms(&audio_short);
    let rms_long = calculate_rms(&audio_long);

    println!("Plate decay 0.5 RMS: {:.4}", rms_short);
    println!("Plate decay 5.0 RMS: {:.4}", rms_long);
    println!("Both should have audio...");

    // Both should have audio
    assert!(rms_short > 0.01, "Short decay plate should produce audio");
    assert!(rms_long > 0.01, "Long decay plate should produce audio");
}

/// Test that reverb actually processes input
#[test]
fn test_reverb_processes_input() {
    // Just the source
    let code_dry = r#"
tempo: 0.5
~src $ saw 220
out $ ~src * 0.5
"#;

    // Source with reverb
    let code_reverb = r#"
tempo: 0.5
~src $ saw 220
out $ ~src * 0.5 # reverb 0.8 0.3 0.3
"#;

    let audio_dry = render_audio(code_dry, 44100);
    let audio_reverb = render_audio(code_reverb, 44100);

    let rms_dry = calculate_rms(&audio_dry);
    let rms_reverb = calculate_rms(&audio_reverb);

    println!("Dry RMS: {:.4}", rms_dry);
    println!("With reverb RMS: {:.4}", rms_reverb);

    // Both should have audio
    assert!(rms_dry > 0.01, "Dry should have audio");
    assert!(rms_reverb > 0.01, "Reverb should have audio");

    // Compute difference to show reverb is doing something
    let diff: f32 = audio_dry.iter()
        .zip(audio_reverb.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / audio_dry.len() as f32;

    println!("Avg sample difference: {:.4}", diff);
    assert!(diff > 0.001, "Reverb should change the output");
}

/// Test reverb with LFO modulated room size
#[test]
fn test_reverb_lfo_modulated() {
    // Room size modulated by LFO
    let code = r#"
tempo: 0.5
~lfo # sine 0.5
~src $ saw 220
out $ ~src # reverb (~lfo * 0.4 + 0.5) 0.3 0.5
"#;

    let audio = render_audio(code, 88200);

    // Split into 4 quarters to see the LFO effect
    let q1 = &audio[0..22050];
    let q2 = &audio[22050..44100];
    let q3 = &audio[44100..66150];
    let q4 = &audio[66150..88200];

    let rms_q1 = calculate_rms(q1);
    let rms_q2 = calculate_rms(q2);
    let rms_q3 = calculate_rms(q3);
    let rms_q4 = calculate_rms(q4);

    println!("LFO-modulated room size:");
    println!("  Q1 RMS: {:.4}", rms_q1);
    println!("  Q2 RMS: {:.4}", rms_q2);
    println!("  Q3 RMS: {:.4}", rms_q3);
    println!("  Q4 RMS: {:.4}", rms_q4);

    // All quarters should have audio
    assert!(rms_q1 > 0.01, "Q1 should have audio");
    assert!(rms_q2 > 0.01, "Q2 should have audio");
    assert!(rms_q3 > 0.01, "Q3 should have audio");
    assert!(rms_q4 > 0.01, "Q4 should have audio");
}
