//! Test envelope support for oscillators

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

// ========== Basic Envelope Tests ==========

#[test]
fn test_envelope_basic() {
    let code = r#"
tempo: 2.0
out: sine 440 # env 0.01 0.1 0.7 0.2
"#;

    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert_eq!(rest.trim(), "", "Parser should consume all input");

    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    // Render 1 second
    let buffer = graph.render(44100);

    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    eprintln!("Enveloped sine RMS: {}", rms);

    assert!(rms > 0.01, "Enveloped oscillator should produce audio");
}

#[test]
fn test_envelope_all_waveforms() {
    // Test that envelope works with all basic waveforms
    let waveforms = vec!["sine", "saw", "square", "tri"];

    for waveform in waveforms {
        let code = format!(
            r#"
tempo: 2.0
out: {} 220 # env 0.01 0.1 0.7 0.2
"#,
            waveform
        );

        let (_, statements) = parse_program(&code).expect("Failed to parse");
        let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
        graph.set_cps(2.0);

        let buffer = graph.render(44100);
        let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

        assert!(
            rms > 0.01,
            "Enveloped {} should produce audio, got RMS={}",
            waveform,
            rms
        );
    }
}

#[test]
fn test_envelope_short_attack() {
    // Test with very short attack (plucky sound)
    let code = r#"
tempo: 2.0
out: sine 440 # env 0.001 0.05 0.0 0.1
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(22050); // 0.5 seconds

    // Check that attack is fast (peak in first few samples)
    let attack_samples = 100; // ~2ms
    let max_attack = buffer[..attack_samples]
        .iter()
        .map(|x| x.abs())
        .fold(0.0f32, f32::max);

    assert!(max_attack > 0.1, "Fast attack should reach high amplitude quickly");
}

#[test]
fn test_envelope_long_attack() {
    // Test with slow attack (pad sound)
    let code = r#"
tempo: 2.0
out: saw 110 # env 0.5 0.2 0.8 0.3
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second

    // Check that attack is gradual
    let early_sample = buffer[4410].abs(); // 100ms in
    let late_sample = buffer[22050].abs(); // 500ms in

    // Later in attack should be louder than early
    assert!(
        late_sample > early_sample * 1.5,
        "Slow attack should gradually increase amplitude"
    );
}

// ========== Envelope Shaping Tests ==========

#[test]
fn test_envelope_zero_sustain() {
    // Percussive envelope with zero sustain (like a kick or pluck)
    let code = r#"
tempo: 2.0
out: sine 440 # env 0.001 0.1 0.0 0.05
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(11025); // 0.25 seconds

    // Should decay to near-silence
    let tail_start = 8820; // After 200ms
    let max_tail = buffer[tail_start..]
        .iter()
        .map(|x| x.abs())
        .fold(0.0f32, f32::max);

    // With zero sustain and short release, should be quiet in tail
    assert!(max_tail < 0.1, "Zero-sustain envelope should decay to low level");
}

#[test]
fn test_envelope_full_sustain() {
    // Organ-like envelope with full sustain
    let code = r#"
tempo: 2.0
out: square 220 # env 0.01 0.05 1.0 0.1
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second

    // Check that sustain is maintained
    let mid_section = &buffer[22050..33075]; // 0.5s to 0.75s
    let mid_rms: f32 = (mid_section.iter().map(|x| x * x).sum::<f32>() / mid_section.len() as f32).sqrt();

    assert!(mid_rms > 0.5, "Full sustain should maintain high level");
}

// ========== Bus and Chaining Tests ==========

#[test]
fn test_envelope_in_bus() {
    let code = r#"
tempo: 2.0
~shaped: sine 440 # env 0.01 0.1 0.7 0.2
out: ~shaped * 0.5
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Enveloped signal in bus should work");
}

#[test]
fn test_envelope_then_filter() {
    // Envelope -> Filter chain
    let code = r#"
tempo: 2.0
out: saw 110 # env 0.01 0.2 0.6 0.3 # lpf 2000 0.8
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Envelope -> filter chain should work");
}

#[test]
fn test_filter_then_envelope() {
    // Filter -> Envelope chain (different order)
    let code = r#"
tempo: 2.0
~filtered: saw 110 # lpf 2000 0.8
out: ~filtered # env 0.01 0.2 0.6 0.3
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Filter -> envelope chain should work");
}

#[test]
fn test_envelope_with_effects() {
    // Complete effects chain with envelope
    let code = r#"
tempo: 2.0
out: sine 440 # env 0.01 0.1 0.7 0.2 # distortion 2.0 0.3 # reverb 0.5 0.5 0.3
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(22050); // 0.5 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Envelope with full effects chain should work");
}

// ========== Musical Use Cases ==========

#[test]
fn test_pluck_sound() {
    // Guitar/pluck-like envelope
    let code = r#"
tempo: 2.0
~pluck: sine 220 # env 0.001 0.3 0.0 0.1
out: ~pluck * 0.6
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(22050); // 0.5 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Pluck envelope should produce audio");
}

#[test]
fn test_pad_sound() {
    // Pad/string-like envelope
    let code = r#"
tempo: 2.0
~pad: saw 110 # env 0.5 0.3 0.8 0.4
out: ~pad * 0.3
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.1, "Pad envelope should produce sustained audio");
}

#[test]
fn test_bass_sound() {
    // Bass synth with envelope
    let code = r#"
tempo: 2.0
~bass: saw 55 # env 0.001 0.2 0.3 0.1 # lpf 800 1.2
out: ~bass * 0.5
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(22050); // 0.5 seconds
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Bass with envelope should produce audio");
}

#[test]
fn test_mixed_enveloped_oscillators() {
    // Multiple oscillators with different envelopes
    let code = r#"
tempo: 2.0
~lead: sine 880 # env 0.001 0.1 0.0 0.05
~pad: saw 220 # env 0.3 0.2 0.7 0.4
out: ~lead * 0.4 + ~pad * 0.3
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(44100); // 1 second
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.01, "Mixed enveloped oscillators should work");
}

// ========== Noise with Envelope ==========

#[test]
fn test_noise_with_envelope() {
    // Noise with short envelope (hi-hat style)
    let code = r#"
tempo: 2.0
~hh: noise 0 # env 0.001 0.05 0.0 0.02 # hpf 8000 2.0
out: ~hh * 0.4
"#;

    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, 44100.0).expect("Failed to compile");
    graph.set_cps(2.0);

    let buffer = graph.render(4410); // 100ms
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    assert!(rms > 0.001, "Noise with envelope should produce audio");
}
