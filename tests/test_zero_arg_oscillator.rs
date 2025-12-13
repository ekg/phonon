//! Test zero-argument oscillator parsing and compilation
//!
//! Goal: Allow synth templates without explicit frequency:
//! ~lead $ saw # lpf 200 0.5 # ar 0.01 0.3
//! out $ s "~lead" # note "c4"

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Render DSL code using chunk-based processing (like working tests)
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;

    let buffer_size = 128;
    let num_buffers = num_samples / buffer_size;
    let mut full_audio = Vec::with_capacity(num_samples);
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }
    full_audio
}

fn calculate_rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
}

#[test]
fn test_parse_zero_arg_saw() {
    // Test parsing: saw # lpf 200 0.5
    let input = r#"~lead $ saw # lpf 200 0.5
out $ s "~lead" # note "c4"
"#;

    let result = parse_program(input);
    match &result {
        Ok((remaining, statements)) => {
            println!("\n=== Parse Result ===");
            println!("Parsed {} statements", statements.len());
            for (i, stmt) in statements.iter().enumerate() {
                println!("Statement {}: {:?}", i, stmt);
            }
            if !remaining.trim().is_empty() {
                println!("Remaining: {:?}", remaining);
            }
        }
        Err(e) => {
            println!("\n=== Parse Error ===");
            println!("{:?}", e);
        }
    }

    assert!(
        result.is_ok(),
        "Should parse zero-arg oscillator: {:?}",
        result.err()
    );
}

#[test]
fn test_compile_zero_arg_saw() {
    let input = r#"~lead $ saw # lpf 200 0.5
out $ s "~lead" # note "c4"
"#;

    let (_, statements) = parse_program(input).expect("Parse should succeed");

    let result = compile_program(statements, 44100.0, None);
    match &result {
        Ok(_graph) => {
            println!("\n=== Compile Success ===");
        }
        Err(e) => {
            println!("\n=== Compile Error ===");
            println!("{}", e);
        }
    }

    assert!(
        result.is_ok(),
        "Should compile zero-arg oscillator: {}",
        result.err().unwrap_or_default()
    );
}

#[test]
fn test_render_zero_arg_saw_with_note() {
    // Test WORKING case first (same pattern as test_note_bus_synth.rs)
    let working_input = r#"~x $ saw 220
out $ s "~x" # note "a3"
"#;

    let audio_working = render_dsl(working_input, 2.0);
    let rms_working = calculate_rms(&audio_working);

    println!("\n=== Working Case (saw 220 without filter) ===");
    println!("RMS: {:.4}", rms_working);

    // Now test zero-arg case
    let zero_arg_input = r#"~x $ saw
out $ s "~x" # note "a3"
"#;

    let audio_zero_arg = render_dsl(zero_arg_input, 2.0);
    let rms_zero_arg = calculate_rms(&audio_zero_arg);

    println!("\n=== Zero-Arg Case (saw) ===");
    println!("RMS: {:.4}", rms_zero_arg);

    // Working case should produce audio
    assert!(
        rms_working > 0.01,
        "Working case should produce audio, got RMS: {}",
        rms_working
    );

    // Zero-arg case should also produce audio since note "a3" provides absolute pitch
    println!(
        "\nComparison: working RMS = {:.4}, zero-arg RMS = {:.4}",
        rms_working, rms_zero_arg
    );

    // Both should produce audio (note provides absolute pitch)
    assert!(
        rms_zero_arg > 0.01,
        "Zero-arg case should produce audio with note, got RMS: {}",
        rms_zero_arg
    );
}

#[test]
fn test_render_zero_arg_saw_with_filter_and_note() {
    // Test the user's desired syntax: saw # lpf # ar # note
    let input_with_filter = r#"~lead $ saw # lpf 800 0.5
out $ s "~lead" # note "c4"
"#;

    let audio = render_dsl(input_with_filter, 2.0);
    let rms = calculate_rms(&audio);

    println!("\n=== Zero-Arg with Filter ===");
    println!("RMS: {:.4}", rms);

    assert!(
        rms > 0.01,
        "Zero-arg saw with filter should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_render_zero_arg_saw_full_chain() {
    // Full chain like user requested: saw # lpf # ar # note
    // Note: ar on the bus doesn't directly work since ar is per-voice
    // For now test lpf only
    let full_chain_input = r#"~lead $ saw # lpf 800 0.5
out $ s "~lead" # note "c4 e4 g4"
"#;

    let audio = render_dsl(full_chain_input, 2.0);
    let rms = calculate_rms(&audio);

    println!("\n=== Full Chain: saw # lpf with note pattern ===");
    println!("RMS: {:.4}", rms);

    assert!(
        rms > 0.01,
        "Full chain should produce audio, got RMS: {}",
        rms
    );
}
