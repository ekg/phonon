/// Systematic tests: ASR Envelope (Attack-Sustain-Release)
///
/// Tests gate-based ASR envelope generator with audio verification.
/// ASR is triggered by a gate signal: Attack (gate rises) → Sustain (gate high) → Release (gate falls)
///
/// Key characteristics:
/// - Gate-triggered (responds to gate signal changes)
/// - Attack: ramps from 0 to 1 when gate goes high
/// - Sustain: holds at 1 while gate remains high
/// - Release: falls from 1 to 0 when gate goes low
/// - Can be re-triggered during release phase
/// - All parameters pattern-modulated
/// - Used for organ-style sounds and continuous notes

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ========== Basic ASR Tests ==========

#[test]
fn test_asr_compiles() {
    let code = r#"
        tempo: 1.0
        ~gate: line 0 1
        ~env: asr ~gate 0.1 0.2
        o1: ~env
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "ASR should compile: {:?}", result.err());
}

#[test]
fn test_asr_generates_envelope() {
    let code = r#"
        tempo: 1.0
        ~gate: line 0 1
        ~env: asr ~gate 0.1 0.2
        o1: ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "ASR should produce envelope, got RMS: {}", rms);
    println!("ASR RMS: {}", rms);
}

// ========== Gate Response Tests ==========

#[test]
fn test_asr_attack_on_gate_rise() {
    // Gate rises from 0 to 1 over 0.5s, ASR should attack
    let code = r#"
        tempo: 1.0
        ~gate: line 0 1
        ~env: asr ~gate 0.2 0.1
        o1: ~env
    "#;

    let _sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    // Start should be low (gate not yet high)
    let start_samples = &buffer[0..100];
    let start_avg: f32 = start_samples.iter().sum::<f32>() / start_samples.len() as f32;
    assert!(start_avg < 0.3, "Start should be low before gate rises, got {}", start_avg);

    // End should be high (gate high, sustain phase)
    let end = buffer.len() - 1000;
    let end_samples = &buffer[end..];
    let end_avg: f32 = end_samples.iter().sum::<f32>() / end_samples.len() as f32;
    assert!(end_avg > 0.7, "End should be high (sustain), got {}", end_avg);

    println!("Gate rise - start: {}, end: {}", start_avg, end_avg);
}

#[test]
fn test_asr_sustain_while_gate_high() {
    // Constant high gate should produce sustain
    let code = r#"
        tempo: 1.0
        ~gate: 1.0
        ~env: asr ~gate 0.05 0.05
        o1: ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    // After attack (at 0.1s), should be at sustain level (1.0)
    let sustain_start = (0.1 * sample_rate) as usize;
    let mid_sustain = (0.5 * sample_rate) as usize;
    let end_sustain = (0.9 * sample_rate) as usize;

    let start_avg: f32 = buffer[sustain_start..sustain_start+1000].iter().sum::<f32>() / 1000.0;
    let mid_avg: f32 = buffer[mid_sustain..mid_sustain+1000].iter().sum::<f32>() / 1000.0;
    let end_avg: f32 = buffer[end_sustain..end_sustain+1000].iter().sum::<f32>() / 1000.0;

    assert!(start_avg > 0.9, "Early sustain should be ~1.0, got {}", start_avg);
    assert!(mid_avg > 0.9, "Mid sustain should be ~1.0, got {}", mid_avg);
    assert!(end_avg > 0.9, "Late sustain should be ~1.0, got {}", end_avg);

    println!("Sustain - start: {}, mid: {}, end: {}", start_avg, mid_avg, end_avg);
}

// ========== Attack Phase Tests ==========

#[test]
fn test_asr_attack_time() {
    // Gate goes high immediately, verify attack time
    let code = r#"
        tempo: 1.0
        ~gate: 1.0
        ~env: asr ~gate 0.3 0.1
        o1: ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    // At 0.15s (halfway through 0.3s attack), should be around 0.5
    let mid_attack = (0.15 * sample_rate) as usize;
    let mid_samples = &buffer[mid_attack..mid_attack + 100];
    let mid_avg: f32 = mid_samples.iter().sum::<f32>() / mid_samples.len() as f32;

    assert!(mid_avg > 0.3 && mid_avg < 0.7,
        "Mid-attack (0.15s into 0.3s) should be ~0.5, got {}",
        mid_avg);

    println!("Mid-attack level: {}", mid_avg);
}

// ========== Release Phase Tests ==========

#[test]
fn test_asr_release_on_gate_fall() {
    // Gate falls from 1 to 0, ASR should release
    let code = r#"
        tempo: 1.0
        ~gate: line 1 0
        ~env: asr ~gate 0.05 0.3
        o1: ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    // After attack completes (at 0.1s), should be high (attack=0.05s, so should be at sustain)
    let after_attack = (0.1 * sample_rate) as usize;
    let sustain_samples = &buffer[after_attack..after_attack + 1000];
    let sustain_avg: f32 = sustain_samples.iter().sum::<f32>() / sustain_samples.len() as f32;
    assert!(sustain_avg > 0.8, "After attack should be at sustain level, got {}", sustain_avg);

    // End should be low (gate fell, release completed)
    let end = buffer.len() - 1000;
    let end_samples = &buffer[end..];
    let end_avg: f32 = end_samples.iter().sum::<f32>() / end_samples.len() as f32;
    assert!(end_avg < 0.3, "End should be low after release, got {}", end_avg);

    println!("Release - sustain: {}, end: {}", sustain_avg, end_avg);
}

// ========== Musical Applications ==========

#[test]
fn test_asr_organ_tone() {
    // Classic organ-style envelope: instant attack, hold, instant release
    let code = r#"
        tempo: 1.0
        ~gate: line 0 1
        ~env: asr ~gate 0.001 0.001
        ~tone: sine 440
        o1: ~tone * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "ASR organ tone should be audible, got RMS: {}", rms);
    println!("Organ tone RMS: {}", rms);
}

#[test]
fn test_asr_pad_sound() {
    // Slow attack and release for pad sounds
    let code = r#"
        tempo: 1.0
        ~gate: 1.0
        ~env: asr ~gate 0.5 0.5
        ~tone: sine 220
        o1: ~tone * ~env * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "ASR pad sound should work, got RMS: {}", rms);
    println!("Pad sound RMS: {}", rms);
}

#[test]
fn test_asr_filter_control() {
    // ASR controlling filter cutoff
    let code = r#"
        tempo: 1.0
        ~gate: line 0 1
        ~env: asr ~gate 0.1 0.2
        ~cutoff: ~env * 3000 + 200
        ~synth: saw 110 # rlpf ~cutoff 2.0
        o1: ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "ASR filter control should work, got RMS: {}", rms);
    println!("ASR filter control RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_asr_pattern_attack() {
    let code = r#"
        tempo: 1.0
        ~gate: 1.0
        ~attack_pat: sine 1 * 0.1 + 0.1
        ~env: asr ~gate ~attack_pat 0.1
        o1: ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "ASR with pattern-modulated attack should work, RMS: {}", rms);
    println!("Pattern-modulated attack RMS: {}", rms);
}

#[test]
fn test_asr_pattern_gate() {
    // Gate modulated by LFO
    let code = r#"
        tempo: 0.5
        ~gate_lfo: sine 2
        ~env: asr ~gate_lfo 0.05 0.05
        o1: ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "ASR with pattern-modulated gate should work, RMS: {}", rms);
    println!("Pattern-modulated gate RMS: {}", rms);
}

// ========== Edge Cases ==========

#[test]
fn test_asr_very_short_times() {
    let code = r#"
        tempo: 1.0
        ~gate: 1.0
        ~env: asr ~gate 0.0001 0.0001
        o1: ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "ASR with very short times should work, got RMS: {}", rms);
    println!("Very short times RMS: {}", rms);
}

#[test]
fn test_asr_long_attack() {
    let code = r#"
        tempo: 1.0
        ~gate: 1.0
        ~env: asr ~gate 0.8 0.1
        o1: ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.15, "ASR with long attack should work, got RMS: {}", rms);
    println!("Long attack RMS: {}", rms);
}
