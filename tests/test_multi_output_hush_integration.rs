/// Comprehensive integration tests for multi-output system (out, o1, o2, etc.),
/// hush/unhush commands, and panic command.
///
/// Tests both the compositional compiler (primary path) and DslCompiler (secondary).
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph_parser::{parse_dsl, DslCompiler, DslStatement};

// ============================================================================
// Helpers
// ============================================================================

fn render_compositional(code: &str, num_samples: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, 44100.0, None).expect("Failed to compile DSL code");
    let buffer_size = 128;
    let num_buffers = num_samples / buffer_size;
    let mut full_audio = Vec::with_capacity(num_samples);
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }
    full_audio
}

fn render_dsl_compiler(code: &str, num_samples: usize) -> Vec<f32> {
    let (_, statements) = parse_dsl(code).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    graph.render(num_samples)
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

// ============================================================================
// Multi-Output: Compositional Compiler
// ============================================================================

#[test]
fn test_out_dollar_syntax() {
    let audio = render_compositional("out $ sine 440 * 0.5", 22050);
    assert!(calculate_rms(&audio) > 0.1, "out $ should produce audio");
}

#[test]
fn test_o1_dollar_syntax() {
    let audio = render_compositional("o1 $ sine 440 * 0.5", 22050);
    assert!(calculate_rms(&audio) > 0.1, "o1 $ should produce audio");
}

#[test]
fn test_d1_dollar_syntax() {
    let audio = render_compositional("d1 $ sine 440 * 0.5", 22050);
    assert!(calculate_rms(&audio) > 0.1, "d1 $ should produce audio");
}

#[test]
fn test_o1_colon_syntax() {
    let audio = render_compositional("o1: sine 440 * 0.5", 22050);
    assert!(calculate_rms(&audio) > 0.1, "o1: should produce audio");
}

#[test]
fn test_multi_channel_dollar() {
    let audio = render_compositional(
        r#"
        o1 $ sine 220 * 0.3
        o2 $ sine 440 * 0.3
        o3 $ sine 880 * 0.3
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) > 0.1,
        "Multiple o$ channels should produce combined audio"
    );
}

#[test]
fn test_mixed_out_and_numbered() {
    let audio = render_compositional(
        r#"
        out $ sine 220 * 0.3
        o2 $ sine 440 * 0.3
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "out $ + o2 $ should both contribute, got RMS: {}",
        rms
    );
}

#[test]
fn test_d_syntax_channels() {
    let audio = render_compositional(
        r#"
        d1 $ sine 220 * 0.3
        d2 $ sine 440 * 0.3
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) > 0.1,
        "d1/d2 syntax should work like o1/o2"
    );
}

// ============================================================================
// Hush: Compositional Compiler
// ============================================================================

#[test]
fn test_hush_all_compositional() {
    let audio = render_compositional(
        r#"
        out $ sine 440 * 0.5
        hush
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) < 0.001,
        "hush should silence all outputs"
    );
}

#[test]
fn test_hush_specific_channel_compositional() {
    let audio = render_compositional(
        r#"
        o1 $ sine 220 * 0.5
        o2 $ sine 440 * 0.5
        hush1
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    // Channel 2 should still be audible
    assert!(
        rms > 0.1,
        "hush1 should only silence channel 1, channel 2 still audible, got RMS: {}",
        rms
    );
}

#[test]
fn test_hush_all_channels_individually() {
    let audio = render_compositional(
        r#"
        o1 $ sine 220 * 0.5
        o2 $ sine 440 * 0.5
        hush1
        hush2
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) < 0.001,
        "hush1 + hush2 should silence all channels"
    );
}

#[test]
fn test_hush_preserves_unhushed_channel() {
    // Hush channel 1, channel 3 should still play
    let audio = render_compositional(
        r#"
        o1 $ sine 110 * 0.3
        o2 $ sine 220 * 0.3
        o3 $ sine 440 * 0.3
        hush1
        hush2
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.05,
        "Channel 3 should still be audible after hush1+hush2, got RMS: {}",
        rms
    );
}

// ============================================================================
// Unhush: Compositional Compiler
// ============================================================================

#[test]
fn test_unhush_all_compositional() {
    let audio = render_compositional(
        r#"
        out $ sine 440 * 0.5
        hush
        unhush
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "unhush should restore audio after hush, got RMS: {}",
        rms
    );
}

#[test]
fn test_unhush_specific_channel() {
    let audio = render_compositional(
        r#"
        o1 $ sine 220 * 0.5
        o2 $ sine 440 * 0.5
        hush
        unhush1
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    // Only channel 1 should be restored
    assert!(
        rms > 0.1,
        "unhush1 should restore channel 1 after hush, got RMS: {}",
        rms
    );
}

#[test]
fn test_unhush_does_nothing_if_not_hushed() {
    let audio_normal = render_compositional("out $ sine 440 * 0.5", 22050);
    let audio_unhush = render_compositional(
        r#"
        out $ sine 440 * 0.5
        unhush
    "#,
        22050,
    );
    let rms_normal = calculate_rms(&audio_normal);
    let rms_unhush = calculate_rms(&audio_unhush);
    assert!(
        (rms_normal - rms_unhush).abs() < 0.01,
        "unhush on non-hushed output should be no-op"
    );
}

// ============================================================================
// Panic: Compositional Compiler
// ============================================================================

#[test]
fn test_panic_silences_synthesis() {
    let audio = render_compositional(
        r#"
        out $ sine 440 * 0.5
        panic
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) < 0.001,
        "panic should silence synthesis outputs"
    );
}

#[test]
fn test_panic_silences_samples() {
    let audio = render_compositional(
        r#"
        tempo: 0.5
        out $ s "bd*4" * 0.5
        panic
    "#,
        44100,
    );
    assert!(
        calculate_rms(&audio) < 0.001,
        "panic should silence sample playback"
    );
}

#[test]
fn test_panic_silences_multi_channel() {
    let audio = render_compositional(
        r#"
        o1 $ sine 220 * 0.5
        o2 $ sine 440 * 0.5
        o3 $ sine 880 * 0.5
        panic
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) < 0.001,
        "panic should silence all channels"
    );
}

// ============================================================================
// DslCompiler Parser Tests
// ============================================================================

#[test]
fn test_dsl_parse_hush() {
    let (_, stmts) = parse_dsl("hush").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        DslStatement::Hush { channel } => assert!(channel.is_none()),
        _ => panic!("Expected Hush"),
    }
}

#[test]
fn test_dsl_parse_hush_channel() {
    let (_, stmts) = parse_dsl("hush3").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        DslStatement::Hush { channel } => assert_eq!(*channel, Some(3)),
        _ => panic!("Expected Hush channel 3"),
    }
}

#[test]
fn test_dsl_parse_unhush() {
    let (_, stmts) = parse_dsl("unhush").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        DslStatement::Unhush { channel } => assert!(channel.is_none()),
        _ => panic!("Expected Unhush"),
    }
}

#[test]
fn test_dsl_parse_unhush_channel() {
    let (_, stmts) = parse_dsl("unhush2").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        DslStatement::Unhush { channel } => assert_eq!(*channel, Some(2)),
        _ => panic!("Expected Unhush channel 2"),
    }
}

#[test]
fn test_dsl_parse_panic() {
    let (_, stmts) = parse_dsl("panic").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        DslStatement::Panic => {}
        _ => panic!("Expected Panic"),
    }
}

// ============================================================================
// DslCompiler Audio Tests
// ============================================================================

#[test]
fn test_dsl_multi_output_colon_syntax() {
    let audio = render_dsl_compiler(
        r#"
        out1: sine 220 * 0.5
        out2: sine 440 * 0.5
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) > 0.1,
        "DslCompiler out1: out2: should produce audio"
    );
}

#[test]
fn test_dsl_hush_all() {
    let audio = render_dsl_compiler(
        r#"
        out $ sine 440 * 0.5
        hush
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) < 0.001,
        "DslCompiler hush should silence output"
    );
}

#[test]
fn test_dsl_hush_channel() {
    let audio = render_dsl_compiler(
        r#"
        out1: sine 220 * 0.5
        out2: sine 440 * 0.5
        hush1
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "DslCompiler hush1 should keep channel 2 audible, got RMS: {}",
        rms
    );
}

#[test]
fn test_dsl_unhush() {
    let audio = render_dsl_compiler(
        r#"
        out $ sine 440 * 0.5
        hush
        unhush
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "DslCompiler unhush should restore audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_dsl_panic() {
    let audio = render_dsl_compiler(
        r#"
        out $ sine 440 * 0.5
        panic
    "#,
        22050,
    );
    assert!(
        calculate_rms(&audio) < 0.001,
        "DslCompiler panic should silence output"
    );
}

// ============================================================================
// Compositional Parser Tests
// ============================================================================

#[test]
fn test_compositional_parse_hush() {
    let (_, stmts) = parse_program("hush").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        phonon::compositional_parser::Statement::Hush { channel } => {
            assert!(channel.is_none())
        }
        _ => panic!("Expected Hush"),
    }
}

#[test]
fn test_compositional_parse_hush_channel() {
    let (_, stmts) = parse_program("hush3").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        phonon::compositional_parser::Statement::Hush { channel } => {
            assert_eq!(*channel, Some(3))
        }
        _ => panic!("Expected Hush channel 3"),
    }
}

#[test]
fn test_compositional_parse_unhush() {
    let (_, stmts) = parse_program("unhush").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        phonon::compositional_parser::Statement::Unhush { channel } => {
            assert!(channel.is_none())
        }
        _ => panic!("Expected Unhush"),
    }
}

#[test]
fn test_compositional_parse_unhush_channel() {
    let (_, stmts) = parse_program("unhush2").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        phonon::compositional_parser::Statement::Unhush { channel } => {
            assert_eq!(*channel, Some(2))
        }
        _ => panic!("Expected Unhush channel 2"),
    }
}

#[test]
fn test_compositional_parse_panic() {
    let (_, stmts) = parse_program("panic").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        phonon::compositional_parser::Statement::Panic => {}
        _ => panic!("Expected Panic"),
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_hush_then_redefine_still_hushed() {
    // In live coding, if you hush and then update the same file,
    // the new compilation should start fresh without hush
    // (because hush is in the DSL, not persisted in the graph)
    let audio = render_compositional(
        r#"
        out $ sine 440 * 0.5
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "Fresh compilation without hush should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_multiple_out_last_wins() {
    // Multiple `out` statements: last one should win
    let audio = render_compositional(
        r#"
        out $ sine 220 * 0.1
        out $ sine 440 * 0.5
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.2,
        "Last out $ should win with higher amplitude, got RMS: {}",
        rms
    );
}

#[test]
fn test_auto_routing_when_no_out() {
    // When there's no explicit out, all buses should be auto-routed
    let audio = render_compositional(
        r#"
        ~drums $ sine 110 * 0.3
        ~bass $ sine 220 * 0.3
    "#,
        22050,
    );
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.1,
        "Auto-routing should mix all buses when no out is specified, got RMS: {}",
        rms
    );
}
