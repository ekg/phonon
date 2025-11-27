/// Systematic tests: AD Envelope (Attack-Decay)
///
/// Tests cycle-based AD envelope generator with audio verification.
/// AD generates one complete envelope per cycle: Attack → Decay → Silent
///
/// Key characteristics:
/// - Cycle-synchronized (one envelope per cycle)
/// - Attack: ramps from 0 to 1 over attack_time
/// - Decay: falls from 1 to 0 over decay_time
/// - After decay: remains at 0 (silent)
/// - No sustain or release phases
/// - All parameters pattern-modulated
/// - Used for percussive, plucked, and transient sounds

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ========== Basic AD Tests ==========

#[test]
fn test_ad_compiles() {
    let code = r#"
        tempo: 1.0
        ~env: ad 0.1 0.2
        o1: ~env
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "AD should compile: {:?}", result.err());
}

#[test]
fn test_ad_generates_envelope() {
    let code = r#"
        tempo: 1.0
        ~env: ad 0.1 0.2
        o1: ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "AD should produce envelope, got RMS: {}", rms);
    println!("AD RMS: {}", rms);
}

// ========== Attack Phase Tests ==========

#[test]
fn test_ad_attack_phase() {
    let code = r#"
        tempo: 1.0
        ~env: ad 0.2 0.3
        o1: ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    let start_samples = &buffer[0..100];
    let start_avg: f32 = start_samples.iter().sum::<f32>() / start_samples.len() as f32;
    assert!(start_avg < 0.1, "Attack should start near 0, got {}", start_avg);

    let mid_attack = (0.1 * sample_rate) as usize;
    let mid_samples = &buffer[mid_attack..mid_attack + 100];
    let mid_avg: f32 = mid_samples.iter().sum::<f32>() / mid_samples.len() as f32;
    assert!(mid_avg > 0.4 && mid_avg < 0.6, "Mid-attack should be ~0.5, got {}", mid_avg);

    let end_attack = (0.2 * sample_rate) as usize;
    let end_samples = &buffer[end_attack..end_attack + 100];
    let end_avg: f32 = end_samples.iter().sum::<f32>() / end_samples.len() as f32;
    assert!(end_avg > 0.9, "End of attack should be near 1.0, got {}", end_avg);

    println!("Attack phase - start: {}, mid: {}, end: {}", start_avg, mid_avg, end_avg);
}

// ========== Decay Phase Tests ==========

#[test]
fn test_ad_decay_phase() {
    let code = r#"
        tempo: 1.0
        ~env: ad 0.05 0.4
        o1: ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    let post_attack = (0.06 * sample_rate) as usize;
    let post_attack_samples = &buffer[post_attack..post_attack + 100];
    let post_attack_avg: f32 = post_attack_samples.iter().sum::<f32>() / post_attack_samples.len() as f32;
    assert!(post_attack_avg > 0.9, "Post-attack should be near 1.0, got {}", post_attack_avg);

    let mid_decay = (0.25 * sample_rate) as usize;
    let mid_samples = &buffer[mid_decay..mid_decay + 100];
    let mid_avg: f32 = mid_samples.iter().sum::<f32>() / mid_samples.len() as f32;
    assert!(mid_avg > 0.4 && mid_avg < 0.7, "Mid-decay should be ~0.5, got {}", mid_avg);

    let end_decay = (0.45 * sample_rate) as usize;
    let end_samples = &buffer[end_decay..end_decay + 100];
    let end_avg: f32 = end_samples.iter().sum::<f32>() / end_samples.len() as f32;
    assert!(end_avg < 0.2, "End of decay should be near 0, got {}", end_avg);

    println!("Decay phase - post-attack: {}, mid: {}, end: {}", post_attack_avg, mid_avg, end_avg);
}

#[test]
fn test_ad_silent_after_decay() {
    let code = r#"
        tempo: 1.0
        ~env: ad 0.1 0.1
        o1: ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    let after_decay = (0.3 * sample_rate) as usize;
    let silent_samples = &buffer[after_decay..];
    let silent_avg: f32 = silent_samples.iter().sum::<f32>() / silent_samples.len() as f32;

    assert!(silent_avg < 0.05, "After decay should be silent, got {}", silent_avg);
    println!("Silent phase average: {}", silent_avg);
}

// ========== Musical Applications ==========

#[test]
fn test_ad_percussive_tone() {
    let code = r#"
        tempo: 0.5
        ~env: ad 0.001 0.15
        ~tone: sine 440
        o1: ~tone * ~env * 0.5
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "AD percussive tone should be audible, got RMS: {}", rms);
    println!("Percussive tone RMS: {}", rms);
}

#[test]
fn test_ad_filter_envelope() {
    let code = r#"
        tempo: 1.0
        ~env: ad 0.05 0.3
        ~cutoff: ~env * 3000 + 200
        ~synth: saw 110 # rlpf ~cutoff 2.0
        o1: ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.03, "AD filter envelope should work, got RMS: {}", rms);
    println!("AD filter envelope RMS: {}", rms);
}

#[test]
fn test_ad_pattern_attack() {
    let code = r#"
        tempo: 0.5
        ~attack_pat: sine 1 * 0.05 + 0.05
        ~env: ad ~attack_pat 0.2
        o1: ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.05, "AD with pattern-modulated attack should work, RMS: {}", rms);
    println!("Pattern-modulated attack RMS: {}", rms);
}

#[test]
fn test_ad_multiple_cycles() {
    let code = r#"
        tempo: 4.0
        ~env: ad 0.05 0.1
        o1: ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);
    let cycle_length = (0.25 * sample_rate) as usize;

    for cycle in 0..4 {
        let start = cycle * cycle_length;
        let end = start + cycle_length;
        let cycle_buffer = &buffer[start..end];

        let first_tenth = &cycle_buffer[0..cycle_length/10];
        let last_half = &cycle_buffer[cycle_length/2..];

        let start_avg: f32 = first_tenth.iter().sum::<f32>() / first_tenth.len() as f32;
        let end_avg: f32 = last_half.iter().sum::<f32>() / last_half.len() as f32;

        assert!(start_avg < 0.8, "Cycle {} start should be in attack phase, got {}", cycle, start_avg);
        assert!(end_avg < 0.3, "Cycle {} end should be post-decay, got {}", cycle, end_avg);

        println!("Cycle {} - start: {}, end: {}", cycle, start_avg, end_avg);
    }
}

#[test]
fn test_ad_very_short_times() {
    let code = r#"
        tempo: 1.0
        ~env: ad 0.001 0.001
        o1: ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.001, "AD with very short times should work, got RMS: {}", rms);
    println!("Very short times RMS: {}", rms);
}
