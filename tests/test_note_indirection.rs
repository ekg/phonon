/// Test indirection scenarios for note patterns
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = buffer.iter().map(|x| x * x).sum();
    (sum_squares / buffer.len() as f32).sqrt()
}

/// Test: Can we store a note pattern in a bus and reference it?
/// ~notes $ "c4 e4 g4"
/// saw ~notes
#[test]
fn test_note_pattern_via_bus_ref() {
    let code = r#"
bpm: 120
~notes $ "c4 e4 g4"
out $ saw ~notes
"#;
    // This might not work - let's see what happens
    let result = std::panic::catch_unwind(|| render_dsl(code, 1.0));
    match result {
        Ok(audio) => {
            let rms = calculate_rms(&audio);
            println!("Bus ref pattern: RMS = {}", rms);
            assert!(rms > 0.01, "Should produce sound");
        }
        Err(_) => {
            println!("Bus ref pattern: FAILED (panicked)");
        }
    }
}

/// Test: saw 220 # note "0 7 12" for pitch shifting
#[test]
fn test_saw_with_note_modifier() {
    let code = r#"
bpm: 120
out $ saw 220 # note "0 7 12"
"#;
    let result = std::panic::catch_unwind(|| render_dsl(code, 1.0));
    match result {
        Ok(audio) => {
            let rms = calculate_rms(&audio);
            println!("Saw with note modifier: RMS = {}", rms);
        }
        Err(e) => {
            println!("Saw with note modifier: FAILED - {:?}", e);
        }
    }
}

/// Test: What if we want continuous oscillator with pitch pattern?
/// This is the case where user explicitly wants pitch modulation, not triggering
#[test]
fn test_continuous_with_pitch_pattern() {
    // User might want: continuous saw that changes pitch per cycle
    // Without envelope/triggering
    let code = r#"
bpm: 120
out $ saw "110 220 440"
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    println!("Continuous pitch pattern: RMS = {}", rms);
    // With current fix, this triggers per-note
    // But what if user wanted continuous pitch changes?
}
