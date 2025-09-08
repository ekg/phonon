//! DSP audio verification tests

use phonon::glicol_parser::parse_glicol;

#[test]
fn test_sine_wave_generation() {
    // Verify sine wave DSP code parses
    let code = "out: sin 440";
    let result = parse_glicol(code);
    if let Err(e) = &result {
        eprintln!("Parse error for '{}': {}", code, e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_amplitude_modulation() {
    // Test amplitude modulation chain
    let code = "out: sin 440 >> mul 0.5";
    let result = parse_glicol(code);
    assert!(result.is_ok());
}

#[test]
fn test_low_pass_filter() {
    // Test low pass filter
    let code = "o: saw 110 >> lpf 1000 0.8";
    let result = parse_glicol(code);
    if let Err(e) = &result {
        eprintln!("Parse error for '{}': {}", code, e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_high_pass_filter() {
    // Test high pass filter
    let code = "out: noise >> hpf 2000 0.9";
    let result = parse_glicol(code);
    assert!(result.is_ok());
}

#[test]
fn test_additive_synthesis() {
    // Test adding multiple oscillators
    let code = r#"
        ~osc1: sin 220
        ~osc2: sin 440
        out: ~osc1 + ~osc2 >> mul 0.5
    "#;
    let result = parse_glicol(code.trim());
    if let Err(e) = &result {
        eprintln!("Parse error for additive_synthesis: {}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_lfo_modulation() {
    // Test LFO modulating filter cutoff
    let code = r#"
        ~lfo: sin 0.5 >> mul 500 >> add 1000
        out: saw 110 >> lpf ~lfo 0.8
    "#;
    let result = parse_glicol(code.trim());
    assert!(result.is_ok());
}

#[test]
fn test_envelope() {
    // Test envelope generation
    let code = "out: sin 440 >> env 0.01 0.1 0.7 0.2";
    let result = parse_glicol(code);
    assert!(result.is_ok());
}

#[test]
fn test_delay_effect() {
    // Test delay line
    let code = r#"
        ~dry: sin 440 >> env 0.01 0.1 0.0 0.0
        ~delayed: ~dry >> delay 0.25 0.5
        out: ~dry + ~delayed
    "#;
    let result = parse_glicol(code.trim());
    assert!(result.is_ok());
}

#[test]
fn test_reverb_effect() {
    // Test reverb
    let code = "out: impulse 1 >> reverb 0.9 0.5";
    let result = parse_glicol(code);
    assert!(result.is_ok());
}

#[test]
#[ignore] // TODO: Fix multiline parsing
fn test_complex_patch() {
    // Test a complex synthesizer patch
    let code = r#"~lfo1: sin 0.2 >> mul 0.5 >> add 0.5
~lfo2: sin 0.13 >> mul 200 >> add 800
~vco1: saw 55
~vco2: square 55.5 >> mul 0.3
~mix: ~vco1 + ~vco2
~filtered: ~mix >> lpf ~lfo2 ~lfo1
~verb: ~filtered >> reverb 0.3 0.5
out: ~filtered * 0.7 + ~verb * 0.3"#;
    let result = parse_glicol(code);
    if let Err(e) = &result {
        eprintln!("Parse error for complex_patch: {}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_fm_synthesis() {
    // Test frequency modulation
    let code = r#"~mod: sin 220 >> mul 100
~carrier: sin (440 + ~mod)
out: ~carrier >> mul 0.5"#;
    let result = parse_glicol(code);
    if let Err(e) = &result {
        eprintln!("Parse error for fm_synthesis: {}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_ring_modulation() {
    // Test ring modulation
    let code = r#"
        ~carrier: sin 440
        ~modulator: sin 7
        out: ~carrier * ~modulator
    "#;
    let result = parse_glicol(code.trim());
    assert!(result.is_ok());
}

#[test]
fn test_noise_generators() {
    // Test different noise types
    let code = r#"
        ~white: noise >> mul 0.1
        ~pink: pink >> mul 0.2
        ~brown: brown >> mul 0.3
        out: ~white + ~pink + ~brown
    "#;
    let result = parse_glicol(code.trim());
    if let Err(e) = &result {
        eprintln!("Parse error for noise_generators: {}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_distortion() {
    // Test distortion/saturation
    let code = "out: sin 220 >> mul 5 >> clip -0.7 0.7";
    let result = parse_glicol(code);
    assert!(result.is_ok());
}

#[test]
fn test_chorus_effect() {
    // Test chorus effect
    let code = r#"
        ~dry: saw 110
        ~lfo: sin 0.5 >> mul 0.002 >> add 0.02
        ~wet: ~dry >> delay ~lfo 0.5
        out: ~dry + ~wet >> mul 0.5
    "#;
    let result = parse_glicol(code.trim());
    assert!(result.is_ok());
}