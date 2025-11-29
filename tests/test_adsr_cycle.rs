/// Systematic tests: ADSR Envelope (Cycle-Based)
///
/// Tests cycle-based ADSR envelope generator with audio verification.
/// ADSR generates one complete envelope per cycle: Attack → Decay → Sustain → Release
///
/// Key characteristics:
/// - Cycle-synchronized (one envelope per cycle)
/// - Attack: ramps from 0 to 1 over attack_time
/// - Decay: falls from 1 to sustain_level over decay_time
/// - Sustain: holds at sustain_level until release starts
/// - Release: falls from sustain_level to 0 over release_time
/// - All parameters pattern-modulated
/// - Used for shaping synthesizer sounds
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

// ========== Basic ADSR Tests ==========

#[test]
fn test_adsr_compiles() {
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.1 0.1 0.5 0.2
        out $ ~env
    "#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let result = compile_program(statements, 44100.0, None);
    assert!(result.is_ok(), "ADSR should compile: {:?}", result.err());
}

#[test]
fn test_adsr_generates_envelope() {
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.1 0.1 0.5 0.2
        out $ ~env
    "#;

    let buffer = render_dsl(code, 1.0); // 1 cycle at tempo 1.0 = 1 second
    let rms = calculate_rms(&buffer);

    assert!(rms > 0.1, "ADSR should produce envelope, got RMS: {}", rms);
    println!("ADSR RMS: {}", rms);
}

// ========== Attack Phase Tests ==========

#[test]
fn test_adsr_attack_phase() {
    // Attack = 0.2s in a 1-second cycle
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.2 0.1 0.7 0.1
        out $ ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    // Start of cycle should be near 0
    let start_samples = &buffer[0..100];
    let start_avg: f32 = start_samples.iter().sum::<f32>() / start_samples.len() as f32;
    assert!(
        start_avg < 0.1,
        "Attack should start near 0, got {}",
        start_avg
    );

    // Middle of attack (0.1s) should be around 0.5
    let mid_attack = (0.1 * sample_rate) as usize;
    let mid_samples = &buffer[mid_attack..mid_attack + 100];
    let mid_avg: f32 = mid_samples.iter().sum::<f32>() / mid_samples.len() as f32;
    assert!(
        mid_avg > 0.4 && mid_avg < 0.6,
        "Mid-attack should be ~0.5, got {}",
        mid_avg
    );

    // End of attack (0.2s) should be near 1.0
    let end_attack = (0.2 * sample_rate) as usize;
    let end_samples = &buffer[end_attack..end_attack + 100];
    let end_avg: f32 = end_samples.iter().sum::<f32>() / end_samples.len() as f32;
    assert!(
        end_avg > 0.9,
        "End of attack should be near 1.0, got {}",
        end_avg
    );

    println!(
        "Attack phase - start: {}, mid: {}, end: {}",
        start_avg, mid_avg, end_avg
    );
}

// ========== Decay Phase Tests ==========

#[test]
fn test_adsr_decay_phase() {
    // Instant attack, 0.3s decay to 0.4 sustain
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.001 0.3 0.4 0.1
        out $ ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    // After attack (at 0.05s), should be in early decay phase
    // Attack=0.001s, so at 0.05s we're 0.049s into 0.3s decay (16% through)
    // Expected: 1.0 - (1.0-0.4)*0.163 ≈ 0.90
    let post_attack = (0.05 * sample_rate) as usize;
    let post_attack_samples = &buffer[post_attack..post_attack + 100];
    let post_attack_avg: f32 =
        post_attack_samples.iter().sum::<f32>() / post_attack_samples.len() as f32;
    assert!(
        post_attack_avg > 0.85 && post_attack_avg <= 1.0,
        "Post-attack should be in early decay (0.85-1.0), got {}",
        post_attack_avg
    );

    // Mid-decay (at 0.15s) should be between 1.0 and 0.4
    let mid_decay = (0.15 * sample_rate) as usize;
    let mid_samples = &buffer[mid_decay..mid_decay + 100];
    let mid_avg: f32 = mid_samples.iter().sum::<f32>() / mid_samples.len() as f32;
    assert!(
        mid_avg > 0.5 && mid_avg < 0.9,
        "Mid-decay should be between 0.5-0.9, got {}",
        mid_avg
    );

    // After decay (at 0.35s) should be near sustain level (0.4)
    let post_decay = (0.35 * sample_rate) as usize;
    let post_decay_samples = &buffer[post_decay..post_decay + 100];
    let post_decay_avg: f32 =
        post_decay_samples.iter().sum::<f32>() / post_decay_samples.len() as f32;
    assert!(
        (post_decay_avg - 0.4).abs() < 0.15,
        "Post-decay should be near sustain (0.4), got {}",
        post_decay_avg
    );

    println!(
        "Decay phase - post-attack: {}, mid: {}, post-decay: {}",
        post_attack_avg, mid_avg, post_decay_avg
    );
}

// ========== Sustain Phase Tests ==========

#[test]
fn test_adsr_sustain_phase() {
    // Test that sustain holds constant
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.1 0.1 0.6 0.2
        out $ ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    // Sustain phase should be from 0.2s to 0.8s (before release)
    let sustain_start = (0.25 * sample_rate) as usize;
    let sustain_mid = (0.5 * sample_rate) as usize;
    let sustain_end = (0.75 * sample_rate) as usize;

    let start_samples = &buffer[sustain_start..sustain_start + 100];
    let start_avg: f32 = start_samples.iter().sum::<f32>() / start_samples.len() as f32;

    let mid_samples = &buffer[sustain_mid..sustain_mid + 100];
    let mid_avg: f32 = mid_samples.iter().sum::<f32>() / mid_samples.len() as f32;

    let end_samples = &buffer[sustain_end..sustain_end + 100];
    let end_avg: f32 = end_samples.iter().sum::<f32>() / end_samples.len() as f32;

    // All three should be near sustain level (0.6)
    assert!(
        (start_avg - 0.6).abs() < 0.15,
        "Sustain start should be ~0.6, got {}",
        start_avg
    );
    assert!(
        (mid_avg - 0.6).abs() < 0.15,
        "Sustain mid should be ~0.6, got {}",
        mid_avg
    );
    assert!(
        (end_avg - 0.6).abs() < 0.15,
        "Sustain end should be ~0.6, got {}",
        end_avg
    );

    println!(
        "Sustain phase - start: {}, mid: {}, end: {}",
        start_avg, mid_avg, end_avg
    );
}

#[test]
fn test_adsr_different_sustain_levels() {
    for sustain_level in [0.2, 0.5, 0.8] {
        let code = format!(
            r#"
            tempo: 1.0
            ~env $ adsr 0.1 0.1 {} 0.2
            out $ ~env
        "#,
            sustain_level
        );

        let sample_rate = 44100.0;
        let buffer = render_dsl(&code, 1.0);

        // Check sustain phase at 0.5s
        let sustain_time = (0.5 * sample_rate) as usize;
        let samples = &buffer[sustain_time..sustain_time + 1000];
        let avg: f32 = samples.iter().sum::<f32>() / samples.len() as f32;

        assert!(
            (avg - sustain_level).abs() < 0.15,
            "Sustain level {} should measure ~{}, got {}",
            sustain_level,
            sustain_level,
            avg
        );

        println!("Sustain level {} - measured: {}", sustain_level, avg);
    }
}

// ========== Release Phase Tests ==========

#[test]
fn test_adsr_release_phase() {
    // Short attack/decay/sustain, 0.3s release
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.05 0.05 0.7 0.3
        out $ ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0);

    // Release starts at (1.0 - 0.3) = 0.7s
    let release_start = (0.7 * sample_rate) as usize;
    let start_samples = &buffer[release_start..release_start + 100];
    let start_avg: f32 = start_samples.iter().sum::<f32>() / start_samples.len() as f32;

    // Should start from sustain level (0.7)
    assert!(
        start_avg > 0.5,
        "Release should start from sustain level, got {}",
        start_avg
    );

    // Mid-release (at 0.85s) should be falling
    let mid_release = (0.85 * sample_rate) as usize;
    let mid_samples = &buffer[mid_release..mid_release + 100];
    let mid_avg: f32 = mid_samples.iter().sum::<f32>() / mid_samples.len() as f32;

    assert!(
        mid_avg < start_avg && mid_avg > 0.1,
        "Mid-release should be falling, start: {}, mid: {}",
        start_avg,
        mid_avg
    );

    // End of cycle should be near 0
    let end = buffer.len() - 1000;
    let end_samples = &buffer[end..];
    let end_avg: f32 = end_samples.iter().sum::<f32>() / end_samples.len() as f32;

    assert!(
        end_avg < 0.2,
        "End of release should be near 0, got {}",
        end_avg
    );

    println!(
        "Release phase - start: {}, mid: {}, end: {}",
        start_avg, mid_avg, end_avg
    );
}

// ========== Musical Applications ==========

#[test]
fn test_adsr_shaping_tone() {
    let code = r#"
        tempo: 0.5
        ~env $ adsr 0.01 0.05 0.6 0.1
        ~tone $ sine 440
        out $ ~tone * ~env * 0.5
    "#;

    let buffer = render_dsl(code, 1.0); // 2 cycles
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.05,
        "ADSR-shaped tone should be audible, got RMS: {}",
        rms
    );
    println!("ADSR-shaped tone RMS: {}", rms);
}

#[test]
fn test_adsr_filter_modulation() {
    // ADSR controlling filter cutoff using inline expression
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.05 0.2 0.3 0.2
        ~synth $ saw 110 # rlpf (~env * 2000 + 200) 2.0
        out $ ~synth * 0.3
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.03,
        "ADSR filter modulation should work, got RMS: {}",
        rms
    );
    println!("ADSR filter mod RMS: {}", rms);
}

#[test]
fn test_adsr_percussive_sound() {
    // Fast attack, no sustain, medium decay/release
    let code = r#"
        tempo: 0.5
        ~env $ adsr 0.001 0.05 0.0 0.05
        ~tone $ sine 220
        out $ ~tone * ~env * 0.5
    "#;

    let buffer = render_dsl(code, 1.0); // 2 cycles
    let rms = calculate_rms(&buffer);

    // Percussive envelope (no sustain) produces lower RMS
    assert!(rms > 0.01, "Percussive ADSR should work, got RMS: {}", rms);
    println!("Percussive ADSR RMS: {}", rms);
}

#[test]
fn test_adsr_pad_sound() {
    // Slow attack, long sustain, slow release
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.3 0.1 0.7 0.3
        ~tone $ sine 220
        out $ ~tone * ~env * 0.4
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Pad with long sustain produces higher RMS
    assert!(rms > 0.15, "Pad ADSR should work, got RMS: {}", rms);
    println!("Pad ADSR RMS: {}", rms);
}

// ========== Pattern Modulation Tests ==========

#[test]
fn test_adsr_pattern_attack() {
    // Pattern-modulated attack time using inline expression
    let code = r#"
        tempo: 0.5
        ~env $ adsr (sine 1 * 0.05 + 0.05) 0.05 0.5 0.1
        out $ ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "ADSR with pattern-modulated attack should work, RMS: {}",
        rms
    );

    println!("Pattern-modulated attack RMS: {}", rms);
}

#[test]
fn test_adsr_pattern_sustain() {
    // Pattern-modulated sustain level using inline expression
    let code = r#"
        tempo: 0.5
        ~env $ adsr 0.05 0.05 (sine 1 * 0.3 + 0.5) 0.1
        out $ ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "ADSR with pattern-modulated sustain should work, RMS: {}",
        rms
    );

    println!("Pattern-modulated sustain RMS: {}", rms);
}

// ========== Multi-Cycle Tests ==========

#[test]
fn test_adsr_multiple_cycles() {
    let code = r#"
        tempo: 4.0
        ~env $ adsr 0.05 0.05 0.5 0.05
        out $ ~env
    "#;

    let sample_rate = 44100.0;
    let buffer = render_dsl(code, 1.0); // 4 cycles in 1 second

    // Each cycle is 0.25s
    let cycle_length = (0.25 * sample_rate) as usize;

    // Verify each cycle has similar envelope shape
    for cycle in 0..4 {
        let start = cycle * cycle_length;
        let end = start + cycle_length;
        let cycle_buffer = &buffer[start..end];

        // Check that envelope rises and falls
        let first_quarter = &cycle_buffer[0..cycle_length / 4];
        let last_quarter = &cycle_buffer[3 * cycle_length / 4..];

        let start_avg: f32 = first_quarter.iter().sum::<f32>() / first_quarter.len() as f32;
        let end_avg: f32 = last_quarter.iter().sum::<f32>() / last_quarter.len() as f32;

        // Start should be rising (attack), end should be falling (release)
        assert!(
            start_avg < 0.8,
            "Cycle {} start should be in attack phase, got {}",
            cycle,
            start_avg
        );
        assert!(
            end_avg < 0.5,
            "Cycle {} end should be in release phase, got {}",
            cycle,
            end_avg
        );

        println!("Cycle {} - start: {}, end: {}", cycle, start_avg, end_avg);
    }
}

// ========== Edge Cases ==========

#[test]
fn test_adsr_very_short_times() {
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.001 0.001 0.5 0.001
        out $ ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.1,
        "ADSR with very short times should work, got RMS: {}",
        rms
    );
    println!("Very short times RMS: {}", rms);
}

#[test]
fn test_adsr_long_attack() {
    // Attack longer than half the cycle
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.6 0.1 0.5 0.2
        out $ ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    assert!(
        rms > 0.15,
        "ADSR with long attack should work, got RMS: {}",
        rms
    );
    println!("Long attack RMS: {}", rms);
}

#[test]
fn test_adsr_zero_sustain() {
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.1 0.2 0.0 0.2
        out $ ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Zero sustain means envelope drops to 0 during decay
    assert!(
        rms > 0.05,
        "ADSR with zero sustain should work, got RMS: {}",
        rms
    );
    println!("Zero sustain RMS: {}", rms);
}

#[test]
fn test_adsr_full_sustain() {
    let code = r#"
        tempo: 1.0
        ~env $ adsr 0.1 0.1 1.0 0.2
        out $ ~env
    "#;

    let buffer = render_dsl(code, 1.0);
    let rms = calculate_rms(&buffer);

    // Full sustain (1.0) means envelope stays at peak after attack
    assert!(
        rms > 0.3,
        "ADSR with full sustain should work, got RMS: {}",
        rms
    );
    println!("Full sustain RMS: {}", rms);
}
