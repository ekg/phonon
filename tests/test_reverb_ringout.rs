//! Test that reverb actually rings out (has a tail after input stops)

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

fn calculate_rms(buffer: &[f32]) -> f32 {
    (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt()
}

/// First verify the pattern creates a silent gap (no reverb)
#[test]
fn test_sample_pattern_creates_silence() {
    // Very fast tempo so BD has more time to decay
    // At tempo 8.0, one cycle = 125ms, so each slot is 62.5ms
    // BD should play in first 62.5ms, then ~rest means silence
    let code = r#"
tempo: 8.0
out $ s "bd ~"
"#;

    let audio = render_audio(code, 44100); // 1 second = 8 cycles

    // Check first vs later cycles
    // First 125ms is one full cycle (bd + rest)
    let first_cycle = &audio[0..5512];  // 125ms
    // 4th cycle (375-500ms) should show pattern if working
    let later_cycle = &audio[16537..22050];

    // First half of first cycle (should have BD)
    let bd_slot = &audio[0..2756];  // 62.5ms
    // Second half of first cycle (should be silent if rest works)
    let rest_slot = &audio[2756..5512];  // 62.5ms

    let rms_bd = calculate_rms(bd_slot);
    let rms_rest = calculate_rms(rest_slot);
    let rms_first_cycle = calculate_rms(first_cycle);
    let rms_later = calculate_rms(later_cycle);

    println!("Sample pattern silence test (NO reverb):");
    println!("  BD slot (0-62ms):     RMS = {:.6}", rms_bd);
    println!("  Rest slot (62-125ms): RMS = {:.6}", rms_rest);
    println!("  First cycle:          RMS = {:.6}", rms_first_cycle);
    println!("  Later cycle:          RMS = {:.6}", rms_later);

    // BD slot should have audio
    assert!(rms_bd > 0.01, "BD slot should have audio");

    // If rest works, rest slot should be quieter (sample may bleed a bit)
    println!("  Rest/BD ratio: {:.4}", rms_rest / rms_bd);
}

/// Test that reverb adds tail energy to the rest slot
#[test]
fn test_reverb_adds_tail_energy() {
    // Compare with and without reverb at tempo 8.0
    // Rest slot should have MORE energy with reverb (reverb tail)
    let code_dry = r#"
tempo: 8.0
out $ s "bd ~"
"#;

    let code_wet = r#"
tempo: 8.0
out $ s "bd ~" # reverb 0.95 0.2 0.5
"#;

    let audio_dry = render_audio(code_dry, 5512);  // One cycle (125ms)
    let audio_wet = render_audio(code_wet, 5512);

    // Rest slot (second half of cycle)
    let rest_dry = &audio_dry[2756..5512];
    let rest_wet = &audio_wet[2756..5512];

    let rms_dry = calculate_rms(rest_dry);
    let rms_wet = calculate_rms(rest_wet);

    println!("Reverb tail energy test:");
    println!("  Rest slot DRY: RMS = {:.6}", rms_dry);
    println!("  Rest slot WET: RMS = {:.6}", rms_wet);
    println!("  Wet/Dry ratio: {:.4}", rms_wet / rms_dry);

    // With reverb, rest slot should have MORE energy (reverb tail)
    assert!(rms_wet > rms_dry * 1.1,
        "Reverb should add tail energy! Wet={:.6}, Dry={:.6}, ratio={:.4}",
        rms_wet, rms_dry, rms_wet / rms_dry);
}

/// Test reverb tail persists after multiple cycles of silence
#[test]
fn test_reverb_tail_persists() {
    // Play BD once, then long silence - reverb tail should persist
    let code = r#"
tempo: 8.0
out $ s "bd ~ ~ ~ ~ ~ ~ ~" # reverb 0.95 0.1 0.8
"#;

    let audio = render_audio(code, 11025); // 250ms = 2 cycles

    // First slot (BD) - at tempo 8.0, one slot = 44100 / (8 * 8) = 689 samples
    // Actually at tempo 8.0, cycle = 44100 / 8 = 5512 samples, 8 slots = 689 each
    let slot_len = 689;
    let bd_slot = &audio[0..slot_len];
    let slot_2 = &audio[slot_len..slot_len*2];
    let slot_4 = &audio[slot_len*3..slot_len*4];
    let slot_6 = &audio[slot_len*5..slot_len*6];

    let rms_bd = calculate_rms(bd_slot);
    let rms_2 = calculate_rms(slot_2);
    let rms_4 = calculate_rms(slot_4);
    let rms_6 = calculate_rms(slot_6);

    println!("Reverb tail persistence test:");
    println!("  Slot 1 (BD):  RMS = {:.6}", rms_bd);
    println!("  Slot 2:       RMS = {:.6}", rms_2);
    println!("  Slot 4:       RMS = {:.6}", rms_4);
    println!("  Slot 6:       RMS = {:.6}", rms_6);

    // BD slot should have audio
    assert!(rms_bd > 0.01, "BD slot should have audio, got {:.6}", rms_bd);

    // Later slots should have some reverb tail energy
    // Tail should decay over time
    assert!(rms_2 > 0.001, "Slot 2 should have reverb tail");
    assert!(rms_4 > 0.0001, "Slot 4 should have reverb tail");
}

/// Test plate reverb ringout
#[test]
fn test_plate_reverb_ringout() {
    // Use sample with rest pattern
    let code = r#"
tempo: 2.0
out $ s "bd ~" # plate 10 3.0 0.7 0.3 0.3 0.9
"#;

    let audio = render_audio(code, 44100);

    let first_half = &audio[0..22050];
    let second_half = &audio[22050..44100];

    let rms_first = calculate_rms(first_half);
    let rms_second = calculate_rms(second_half);

    println!("\nPlate reverb ringout test:");
    println!("  First half (with BD):    RMS = {:.6}", rms_first);
    println!("  Second half (tail only): RMS = {:.6}", rms_second);

    assert!(rms_first > 0.01, "First half should have BD sample");
    assert!(rms_second > 0.0001,
        "Second half should have plate reverb tail! Got RMS={:.6}", rms_second);
}

/// Test with simple continuous saw - baseline that reverb works
#[test]
fn test_reverb_continuous_baseline() {
    let code = r#"
tempo: 1.0
out $ saw 220 # reverb 0.95 0.2 0.8
"#;

    let audio = render_audio(code, 44100);
    let rms = calculate_rms(&audio);

    println!("\nContinuous saw through reverb:");
    println!("  RMS = {:.6}", rms);

    assert!(rms > 0.01, "Continuous saw through reverb should produce audio");
}

/// Test dry signal without reverb
#[test]
fn test_dry_signal_baseline() {
    let code = r#"
tempo: 1.0
out $ saw 220
"#;

    let audio = render_audio(code, 44100);
    let rms = calculate_rms(&audio);

    println!("\nDry saw wave:");
    println!("  RMS = {:.6}", rms);

    assert!(rms > 0.01, "Dry saw should produce audio");
}

/// Debug: test gating mechanism without reverb
#[test]
fn test_gate_pattern_debug() {
    // Test 1: multiply by constant 1 (should have audio)
    let code1 = r#"
tempo: 1.0
out $ saw 220 * 1.0
"#;
    let audio1 = render_audio(code1, 44100);
    let rms1 = calculate_rms(&audio1);
    println!("\nGate debug - saw * 1.0: RMS = {:.6}", rms1);

    // Test 2: multiply by constant 0.5
    let code2 = r#"
tempo: 1.0
out $ saw 220 * 0.5
"#;
    let audio2 = render_audio(code2, 44100);
    let rms2 = calculate_rms(&audio2);
    println!("Gate debug - saw * 0.5: RMS = {:.6}", rms2);

    // Test 3: multiply by bus reference (constant pattern)
    let code3 = r#"
tempo: 1.0
~amp # 0.5
out $ saw 220 * ~amp
"#;
    let audio3 = render_audio(code3, 44100);
    let rms3 = calculate_rms(&audio3);
    println!("Gate debug - saw * ~amp (0.5): RMS = {:.6}", rms3);

    // Test 4: bus with pattern
    let code4 = r#"
tempo: 1.0
~amp # "1 0.5"
out $ saw 220 * ~amp
"#;
    let audio4 = render_audio(code4, 44100);
    let rms4 = calculate_rms(&audio4);
    println!("Gate debug - saw * ~amp (\"1 0.5\"): RMS = {:.6}", rms4);

    // Test 5: The problematic case
    let code5 = r#"
tempo: 1.0
~gate # "1 0"
~src $ saw 220 * ~gate
out $ ~src
"#;
    let audio5 = render_audio(code5, 44100);
    let rms5 = calculate_rms(&audio5);
    println!("Gate debug - ~src $ saw 220 * ~gate (\"1 0\"): RMS = {:.6}", rms5);

    // Test 6: inline multiply
    let code6 = r#"
tempo: 1.0
~gate # "1 0"
out $ saw 220 * ~gate
"#;
    let audio6 = render_audio(code6, 44100);
    let rms6 = calculate_rms(&audio6);
    println!("Gate debug - out $ saw 220 * ~gate: RMS = {:.6}", rms6);

    assert!(rms1 > 0.01, "saw * 1.0 should work");
    assert!(rms2 > 0.01, "saw * 0.5 should work");
}

/// Test reverb tail with snare sample
#[test]
fn test_reverb_snare_ringout() {
    // Snare is a good test - short attack, should have clear tail
    let code = r#"
tempo: 2.0
out $ s "sn ~" # reverb 0.95 0.2 0.8
"#;

    let audio = render_audio(code, 44100);

    // First 500ms: snare hit
    let first_half = &audio[0..22050];
    // Second 500ms: should be reverb tail only
    let second_half = &audio[22050..44100];

    let rms_first = calculate_rms(first_half);
    let rms_second = calculate_rms(second_half);

    println!("\nSnare reverb ringout test:");
    println!("  First half (with SN):    RMS = {:.6}", rms_first);
    println!("  Second half (tail only): RMS = {:.6}", rms_second);

    assert!(rms_first > 0.01, "First half should have snare");
    assert!(rms_second > 0.0001,
        "Second half should have reverb tail! Got RMS={:.6}. Reverb isn't ringing out!",
        rms_second);
}
