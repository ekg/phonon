use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

/// LEVEL 1: Pattern Query Verification
/// Tests that ASR syntax is parsed and compiled correctly
#[test]
fn test_asr_pattern_query() {
    let dsl = r#"
tempo: 1.0
~gate: "1.0 0.0"
~envelope: asr ~gate 0.1 0.2
out: ~envelope
"#;

    let (remaining, statements) = parse_program(dsl).unwrap();
    assert!(
        remaining.trim().is_empty(),
        "Should parse completely, remaining: '{}'",
        remaining
    );

    let graph = compile_program(statements, SAMPLE_RATE, None);
    assert!(
        graph.is_ok(),
        "ASR should compile successfully: {:?}",
        graph.err()
    );
}

/// LEVEL 2: ASR Attack Phase
/// Tests that envelope attacks from 0 to 1 when gate goes high
#[test]
fn test_asr_attack() {
    let dsl = r#"
tempo: 1.0
~gate_const: sine 0.0
~gate: ~gate_const * 0.0 + 1.0
~envelope: asr ~gate 0.1 0.1
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render attack phase (0.15 seconds to see attack progress)
    let attack_samples = (SAMPLE_RATE * 0.15) as usize;
    let samples = graph.render(attack_samples);

    // Check that envelope starts near 0
    let start_val = samples[10]; // Skip first few samples for startup
    assert!(start_val < 0.2, "Should start near 0, got {}", start_val);

    // Check that envelope rises
    let mid_val = samples[attack_samples / 2];
    let end_val = samples[attack_samples - 1];

    println!("Start: {}, Mid: {}, End: {}", start_val, mid_val, end_val);

    assert!(
        end_val > start_val + 0.3,
        "Envelope should rise significantly during attack, got start={} end={}",
        start_val,
        end_val
    );
}

/// LEVEL 2: ASR Sustain Phase
/// Tests that envelope holds at peak while gate is high
#[test]
fn test_asr_sustain() {
    let dsl = r#"
tempo: 1.0
~gate: sine 0.0
~gate_high: ~gate * 0.0 + 1.0
~envelope: asr ~gate_high 0.01 0.01
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render enough to get through attack into sustain
    let samples = graph.render((SAMPLE_RATE * 0.1) as usize);

    // Check that late samples are near 1.0 (sustaining)
    let sustain_start = (SAMPLE_RATE * 0.05) as usize;
    let sustain_vals: Vec<f32> = samples[sustain_start..].to_vec();
    let mean_sustain = sustain_vals.iter().sum::<f32>() / sustain_vals.len() as f32;

    println!("Mean sustain level: {}", mean_sustain);

    assert!(
        mean_sustain > 0.8,
        "Envelope should sustain near 1.0, got {}",
        mean_sustain
    );
}

/// LEVEL 2: ASR Release Phase
/// Tests that envelope releases to 0 when gate goes low
#[test]
fn test_asr_release() {
    let dsl = r#"
tempo: 0.5
~gate: "1.0 0.0"
~envelope: asr ~gate 0.01 0.1
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    // Render 1 full cycle (gate high for 0.5s, low for 0.5s)
    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Find where envelope starts releasing (after gate goes low)
    let mid_point = samples.len() / 2;
    let release_start = samples[mid_point];
    let release_end = samples[samples.len() - 1];

    println!(
        "Release start: {}, Release end: {}",
        release_start, release_end
    );

    // Envelope should decay during release
    assert!(
        release_end < release_start,
        "Envelope should decay during release"
    );

    // Should approach 0
    assert!(
        release_end < 0.3,
        "Should release toward 0, got {}",
        release_end
    );
}

/// LEVEL 2: ASR Retrigger
/// Tests that envelope can retrigger from release phase
#[test]
fn test_asr_retrigger() {
    let dsl = r#"
tempo: 4.0
~gate: "1.0 0.0 1.0 0.0"
~envelope: asr ~gate 0.01 0.01
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Check for high values during attack/sustain phases
    // With fast attack (0.01s), we should reach high values
    let high_samples = samples.iter().filter(|&&s| s > 0.7).count();

    println!("Found {} samples above 0.7", high_samples);
    println!("Sample values at key points:");
    println!(
        "  0.05s (first attack): {}",
        samples[(SAMPLE_RATE * 0.05) as usize]
    );
    println!(
        "  0.15s (after first release): {}",
        samples[(SAMPLE_RATE * 0.15) as usize]
    );
    println!(
        "  0.30s (second attack): {}",
        samples[(SAMPLE_RATE * 0.30) as usize]
    );

    assert!(
        high_samples > 100,
        "Should have sustained high envelope values (retriggers), got {} samples above 0.7",
        high_samples
    );
}

/// LEVEL 3: Musical Example - Organ-style tone
/// Tests ASR used for sustained organ sound
#[test]
fn test_asr_organ() {
    let dsl = r#"
tempo: 0.5
~gate: "1.0 0.0 1.0 1.0"
~env: asr ~gate 0.02 0.1
~tone: sine 440
out: ~tone * ~env * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 1.0) as usize);

    // Should produce audible output
    let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;

    assert!(
        rms.sqrt() > 0.05,
        "Organ tone should be audible, got RMS {}",
        rms.sqrt()
    );
}

/// LEVEL 3: ASR with Variable Attack/Release
/// Tests that attack and release times can be patterns
#[test]
fn test_asr_pattern_times() {
    let dsl = r#"
tempo: 1.0
~gate: "1.0 0.0"
~attack: "0.01 0.1"
~release: "0.05 0.2"
~envelope: asr ~gate ~attack ~release
out: ~envelope
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let graph = compile_program(statements, SAMPLE_RATE, None);

    assert!(
        graph.is_ok(),
        "Pattern-modulated ASR should compile: {:?}",
        graph.err()
    );
}

/// LEVEL 3: ASR Gating Synth
/// Tests ASR used for gated synthesis
#[test]
fn test_asr_gating() {
    let dsl = r#"
tempo: 4.0
~gate: "1.0 0.0 1.0 0.0"
~env: asr ~gate 0.01 0.05
~synth: saw 110
~gated: ~synth * ~env
out: ~gated * 0.3
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.5) as usize);

    // Should have distinct note events (not continuous)
    // Check for silence periods (release phases reaching low values)
    let silent_samples = samples.iter().filter(|&&s| s.abs() < 0.01).count();

    println!("Silent samples: {}", silent_samples);

    assert!(
        silent_samples > 100,
        "Should have silence between notes, got {} silent samples",
        silent_samples
    );
}

/// LEVEL 3: ASR Fast Attack, Slow Release
/// Tests asymmetric attack/release times
#[test]
fn test_asr_asymmetric() {
    let dsl = r#"
tempo: 0.5
~gate: "1.0 0.0"
~fast_attack_slow_release: asr ~gate 0.001 0.2
out: ~fast_attack_slow_release
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, SAMPLE_RATE, None).unwrap();

    let samples = graph.render((SAMPLE_RATE * 0.75) as usize);

    // Attack should be fast (reach high value quickly)
    let attack_end = (SAMPLE_RATE * 0.05) as usize;
    let attack_level = samples[attack_end];

    // Release should be slow (still high after gate goes low)
    let release_mid = (SAMPLE_RATE * 0.6) as usize;
    let release_level = samples[release_mid];

    println!("Attack level at 0.05s: {}", attack_level);
    println!("Release level at 0.6s: {}", release_level);

    assert!(
        attack_level > 0.7,
        "Fast attack should reach peak quickly, got {}",
        attack_level
    );

    assert!(
        release_level > 0.1,
        "Slow release should still be audible, got {}",
        release_level
    );
}
