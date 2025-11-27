//! END-TO-END Cross-Mode DSL Tests
//!
//! These tests verify that the SAME Phonon DSL code (.ph file content)
//! works identically across all execution modes.
//!
//! Unlike the Rust API tests, these use actual DSL syntax that users would write.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::osc_live_server::{apply_command_to_graph, LiveCommand};
use std::fs;
use std::process::Command;

/// Reference DSL code that should work in ALL modes
const REFERENCE_DSL: &str = r#"-- Phonon Reference Test
tempo: 2.0
~tone: sine 440 * 0.1
~lfo: sine 0.5 * 0.5 + 0.5
~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8
out: ~tone + ~bass * 0.3
"#;

/// Test that reference DSL renders successfully via phonon render command
#[test]
fn test_dsl_render_mode() {
    // Write DSL to file
    fs::write("/tmp/test_dsl_render.ph", REFERENCE_DSL).unwrap();

    // Render using phonon binary
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "phonon",
            "--quiet",
            "--",
            "render",
            "/tmp/test_dsl_render.ph",
            "/tmp/test_dsl_render.wav",
            "--duration",
            "1",
        ])
        .output()
        .expect("Failed to run phonon render");

    assert!(
        output.status.success(),
        "Render should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify WAV file was created and has audio
    let metadata = fs::metadata("/tmp/test_dsl_render.wav").expect("WAV file should exist");
    assert!(metadata.len() > 1000, "WAV file should have content");

    // Analyze audio
    let analyze_output = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "wav_analyze",
            "--",
            "/tmp/test_dsl_render.wav",
        ])
        .output()
        .expect("Failed to analyze WAV");

    let analysis = String::from_utf8_lossy(&analyze_output.stdout);
    assert!(
        analysis.contains("✅ Contains audio signal"),
        "Should produce audio"
    );
}

/// Test that reference DSL compiles via compositional compiler (used by all modes internally)
#[test]
fn test_dsl_compiler_direct() {
    let (remaining, statements) = parse_program(REFERENCE_DSL).expect("Should parse DSL");

    // Should parse completely (or only have whitespace/comments remaining)
    // Note: compositional parser may leave comments in remaining
    let non_comment_remaining = remaining
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        non_comment_remaining.trim().is_empty(),
        "Should parse all input. Remaining: {:?}",
        remaining
    );

    // Compile to graph
    let mut graph = compile_program(statements, 44100.0, None).expect("Should compile");

    // Should have output
    assert!(graph.has_output(), "Graph should have output set");

    // Should set tempo
    assert_eq!(graph.get_cps(), 2.0, "Should set CPS to 2.0");

    // Should produce audio
    let audio = graph.render(44100);
    let has_audio = audio.iter().any(|&s| s.abs() > 0.001);
    assert!(has_audio, "Should produce audio samples");

    // Check audio characteristics
    let rms: f32 = (audio.iter().map(|&s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.01, "RMS should be > 0.01, got {}", rms);
}

/// Test that reference DSL works via OSC /eval command
#[test]
fn test_dsl_osc_mode() {
    let cmd = LiveCommand::Eval {
        code: REFERENCE_DSL.to_string(),
    };

    let graph_opt = apply_command_to_graph(&cmd, 44100.0);
    assert!(graph_opt.is_some(), "OSC /eval should compile DSL");

    let mut graph = graph_opt.unwrap();

    // Should have output
    assert!(graph.has_output(), "OSC graph should have output");

    // Should set tempo
    assert_eq!(graph.get_cps(), 2.0, "OSC should set CPS");

    // Should produce audio
    let audio = graph.render(44100);
    let has_audio = audio.iter().any(|&s| s.abs() > 0.001);
    assert!(has_audio, "OSC mode should produce audio");
}

/// Test audio consistency across modes
#[test]
fn test_dsl_audio_consistency_across_modes() {
    let sample_rate = 44100.0;
    let render_samples = 4410; // 0.1 seconds

    // Mode 1: Direct compositional compiler
    let (_, statements) = parse_program(REFERENCE_DSL).unwrap();
    let mut graph1 = compile_program(statements, sample_rate, None).unwrap();
    let audio1 = graph1.render(render_samples);

    // Mode 2: OSC Server
    let cmd = LiveCommand::Eval {
        code: REFERENCE_DSL.to_string(),
    };
    let mut graph2 = apply_command_to_graph(&cmd, sample_rate).unwrap();
    let audio2 = graph2.render(render_samples);

    // Audio should be similar (allowing for minor floating point differences)
    let rms1: f32 = (audio1.iter().map(|&s| s * s).sum::<f32>() / audio1.len() as f32).sqrt();
    let rms2: f32 = (audio2.iter().map(|&s| s * s).sum::<f32>() / audio2.len() as f32).sqrt();

    let rms_diff = (rms1 - rms2).abs() / rms1.max(rms2);
    assert!(
        rms_diff < 0.01,
        "RMS should be similar across modes: {} vs {}, diff: {}",
        rms1,
        rms2,
        rms_diff
    );

    println!("✅ Audio consistent across modes:");
    println!("   Compositional Compiler RMS: {:.6}", rms1);
    println!("   OSC Server RMS:  {:.6}", rms2);
    println!("   Difference:      {:.2}%", rms_diff * 100.0);
}

/// Test simple DSL variations
#[test]
fn test_dsl_simple_variations() {
    let test_cases = vec![
        (
            "Simple sine",
            r#"tempo: 2.0
out: sine 440 * 0.2
"#,
        ),
        (
            "Saw wave",
            r#"tempo: 2.0
out: saw 110 * 0.3
"#,
        ),
        (
            "With bus",
            r#"tempo: 2.0
~osc: sine 440
out: ~osc * 0.2
"#,
        ),
        (
            "Simple filter",
            r#"tempo: 2.0
out: saw 110 # lpf 1000 0.8
"#,
        ),
    ];

    for (name, dsl) in test_cases {
        let (remaining, statements) =
            parse_program(dsl).unwrap_or_else(|e| panic!("{}: Failed to parse: {:?}", name, e));

        // Check that only whitespace/comments remain
        let non_comment_remaining = remaining
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            non_comment_remaining.trim().is_empty(),
            "{}: Should parse completely. Remaining: {:?}",
            name,
            remaining
        );

        let mut graph = compile_program(statements, 44100.0, None).unwrap();

        assert!(graph.has_output(), "{}: Should have output", name);

        let audio = graph.render(4410);
        let has_audio = audio.iter().any(|&s| s.abs() > 0.001);
        assert!(has_audio, "{}: Should produce audio", name);

        println!("✅ {}: DSL compiles and produces audio", name);
    }
}

/// Test that auto-routing works end-to-end
#[test]
fn test_dsl_auto_routing() {
    // This should auto-route ~d1 to master output
    let dsl = r#"tempo: 2.0
~d1: sine 440 * 0.2
"#;

    let (_, statements) = parse_program(dsl).expect("Should parse");
    let graph = compile_program(statements, 44100.0, None).expect("Should compile");

    // Auto-routing should set output
    assert!(
        graph.has_output(),
        "Auto-routing should set output from ~d1"
    );

    // Should produce audio
    let mut graph_mut = graph;
    let audio = graph_mut.render(4410);
    let has_audio = audio.iter().any(|&s| s.abs() > 0.001);
    assert!(has_audio, "Auto-routed ~d1 should produce audio");

    println!("✅ Auto-routing works: ~d1 → output");
}

/// Test multi-bus mixing
#[test]
fn test_dsl_multi_bus() {
    let dsl = r#"tempo: 2.0
~bass: saw 55 * 0.3
~lead: sine 440 * 0.2
out: ~bass + ~lead
"#;

    let (_, statements) = parse_program(dsl).expect("Should parse");
    let mut graph = compile_program(statements, 44100.0, None).expect("Should compile");

    assert!(graph.has_output(), "Should have output");

    let audio = graph.render(4410);
    let has_audio = audio.iter().any(|&s| s.abs() > 0.001);
    assert!(has_audio, "Multi-bus mix should produce audio");

    let rms: f32 = (audio.iter().map(|&s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.05, "Mixed signal should have decent level: {}", rms);

    println!("✅ Multi-bus mixing works, RMS: {:.6}", rms);
}
