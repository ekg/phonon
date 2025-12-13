/// Test customizable ADSR for MidiPolySynth
/// Syntax: sine ~midi :attack 0.1 :release 2.0

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Test that the syntax parses without error (can't test MIDI without device)
#[test]
fn test_midi_poly_adsr_syntax_parses() {
    let code = r#"
out $ sine ~midi :attack 0.1 :release 2.0
"#;
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse");

    // This will fail because no MIDI device, but we want to check parse works
    let result = compile_program(statements, sample_rate, None);

    // Should fail with MIDI error, not syntax error
    match result {
        Err(e) => {
            assert!(
                e.contains("MIDI") || e.contains("midi"),
                "Should fail due to MIDI, not syntax. Got: {}",
                e
            );
        }
        Ok(_) => {
            // Surprisingly compiled - maybe test env has MIDI stub
            println!("Compiled successfully");
        }
    }
}

/// Test that default values work (no :attack :release specified)
#[test]
fn test_midi_poly_default_envelope() {
    let code = r#"
out $ sine ~midi
"#;
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse");

    let result = compile_program(statements, sample_rate, None);
    match result {
        Err(e) => {
            assert!(
                e.contains("MIDI") || e.contains("midi"),
                "Should fail due to MIDI, not syntax. Got: {}",
                e
            );
        }
        Ok(_) => {
            println!("Compiled successfully");
        }
    }
}

/// Test with saw waveform
#[test]
fn test_saw_midi_with_long_release() {
    let code = r#"
out $ saw ~midi :attack 0.05 :release 3.0
"#;
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse");

    let result = compile_program(statements, sample_rate, None);
    match result {
        Err(e) => {
            assert!(
                e.contains("MIDI") || e.contains("midi"),
                "Should fail due to MIDI, not syntax. Got: {}",
                e
            );
        }
        Ok(_) => {
            println!("Compiled successfully");
        }
    }
}
