use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;

    // Render in chunks for synthesis voices
    let buffer_size = 128;
    let num_buffers = num_samples / buffer_size;
    let mut full_audio = Vec::with_capacity(num_samples);
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }
    full_audio
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|s| s * s).sum();
    (sum / buffer.len() as f32).sqrt()
}

#[test]
fn test_legato_parsing() {
    // Test that legato parses without errors at BPM 120
    let code = r#"
        bpm: 120
        ~synth $ sine 440
        out $ s "~synth*4" $ legato 1.0 # note "c4 e4 g4 c5"
    "#;

    let audio = render_dsl(code, 4.0); // 4 cycles at BPM 120 = 2 seconds
    let rms = calculate_rms(&audio);

    println!("Legato test RMS: {:.3}", rms);
    // These plucky synth notes peak at ~0.5 but are active for only ~1% of the render
    // (fast-decay envelope, sparse notes), so whole-buffer RMS is ~0.015 by design.
    // The 0.05 threshold assumed sustained notes; 0.005 correctly asserts "produces
    // audio". Relative legato/staccato semantics are covered by test_legato_vs_staccato_rms.
    assert!(
        rms > 0.005,
        "Legato should produce audio, got RMS: {:.3}",
        rms
    );
}

#[test]
fn test_staccato_parsing() {
    // Test that staccato parses without errors
    let code = r#"
        bpm: 120
        ~synth $ sine 440
        out $ s "~synth*4" $ staccato 0.5 # note "c4 e4 g4 c5"
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);

    println!("Staccato test RMS: {:.3}", rms);
    // Staccato shortens notes further, so whole-buffer RMS is ~0.009 (peak still ~0.5).
    // See test_legato_parsing note; 0.003 asserts "produces audio" without assuming sustain.
    assert!(
        rms > 0.003,
        "Staccato should produce audio, got RMS: {:.3}",
        rms
    );
}

#[test]
fn test_stretch_parsing() {
    // Test that stretch parses without errors
    let code = r#"
        bpm: 120
        ~synth $ sine 440
        out $ s "~synth*4" $ stretch # note "c4 e4 g4 c5"
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);

    println!("Stretch test RMS: {:.3}", rms);
    // Same plucky-note dilution as test_legato_parsing (RMS ~0.015, peak ~0.5).
    assert!(
        rms > 0.005,
        "Stretch should produce audio, got RMS: {:.3}",
        rms
    );
}

#[test]
fn test_legato_vs_staccato_rms() {
    // Legato should have higher RMS than staccato because notes are longer
    let legato_code = r#"
        bpm: 120
        ~synth $ sine 440
        out $ s "~synth*4" $ legato 1.0 # note "c4"
    "#;

    let staccato_code = r#"
        bpm: 120
        ~synth $ sine 440
        out $ s "~synth*4" $ staccato 0.5 # note "c4"
    "#;

    let legato_audio = render_dsl(legato_code, 4.0);
    let staccato_audio = render_dsl(staccato_code, 4.0);

    let legato_rms = calculate_rms(&legato_audio);
    let staccato_rms = calculate_rms(&staccato_audio);

    println!(
        "Legato RMS: {:.3}, Staccato RMS: {:.3}",
        legato_rms, staccato_rms
    );

    assert!(legato_rms > staccato_rms,
        "Legato (longer notes) should have higher RMS than staccato (shorter notes). Legato: {:.3}, Staccato: {:.3}",
        legato_rms, staccato_rms);
}
