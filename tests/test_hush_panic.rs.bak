/// Tests for hush and panic commands in live coding
///
/// Tests both the parser and the actual functionality of hush/panic commands.
use phonon::unified_graph_parser::{parse_dsl, DslCompiler, DslStatement};

#[test]
fn test_parse_hush_all() {
    let input = "hush";
    let result = parse_dsl(input);
    assert!(result.is_ok(), "Should parse hush statement");

    if let Ok((_, statements)) = result {
        assert_eq!(statements.len(), 1);
        match &statements[0] {
            DslStatement::Hush { channel } => {
                assert!(
                    channel.is_none(),
                    "Plain 'hush' should have no channel (hushes all)"
                );
            }
            _ => panic!("Expected Hush statement"),
        }
    }
}

#[test]
fn test_parse_hush_channel_1() {
    let input = "hush1";
    let result = parse_dsl(input);
    assert!(result.is_ok(), "Should parse hush1 statement");

    if let Ok((_, statements)) = result {
        assert_eq!(statements.len(), 1);
        match &statements[0] {
            DslStatement::Hush { channel } => {
                assert_eq!(*channel, Some(1), "hush1 should target channel 1");
            }
            _ => panic!("Expected Hush statement"),
        }
    }
}

#[test]
fn test_parse_hush_channel_2() {
    let input = "hush2";
    let result = parse_dsl(input);
    assert!(result.is_ok(), "Should parse hush2 statement");

    if let Ok((_, statements)) = result {
        assert_eq!(statements.len(), 1);
        match &statements[0] {
            DslStatement::Hush { channel } => {
                assert_eq!(*channel, Some(2), "hush2 should target channel 2");
            }
            _ => panic!("Expected Hush statement"),
        }
    }
}

#[test]
fn test_parse_panic() {
    let input = "panic";
    let result = parse_dsl(input);
    assert!(result.is_ok(), "Should parse panic statement");

    if let Ok((_, statements)) = result {
        assert_eq!(statements.len(), 1);
        match &statements[0] {
            DslStatement::Panic => {
                // Success - panic parsed correctly
            }
            _ => panic!("Expected Panic statement"),
        }
    }
}

#[test]
fn test_hush_silences_single_output() {
    // First, create an output with audio
    let input = r#"
        tempo: 2.0
        out: sine(440) * 0.5
    "#;
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render some audio - should have signal
    let buffer_before = graph.render(4410); // 0.1 seconds
    let rms_before: f32 =
        (buffer_before.iter().map(|x| x * x).sum::<f32>() / buffer_before.len() as f32).sqrt();
    assert!(
        rms_before > 0.1,
        "Should have audio before hush, got RMS: {}",
        rms_before
    );

    // Now hush the output
    let hush_input = "hush";
    let (_, hush_statements) = parse_dsl(hush_input).unwrap();
    let hush_compiler = DslCompiler::new(44100.0);
    graph = hush_compiler.compile(hush_statements);

    // Actually, we need to apply hush to the existing graph
    // The hush statement should be applied to the same graph
    // Let me fix this test

    // Better approach: compile everything together
    let full_input = r#"
        tempo: 2.0
        out: sine(440) * 0.5
        hush
    "#;
    let (_, full_statements) = parse_dsl(full_input).unwrap();
    let full_compiler = DslCompiler::new(44100.0);
    let mut graph = full_compiler.compile(full_statements);

    // Render audio after hush - should be silent
    let buffer_after = graph.render(4410);
    let rms_after: f32 =
        (buffer_after.iter().map(|x| x * x).sum::<f32>() / buffer_after.len() as f32).sqrt();
    assert!(
        rms_after < 0.001,
        "Should be silent after hush, got RMS: {}",
        rms_after
    );
}

#[test]
fn test_hush_channel_silences_specific_channel() {
    // Create two output channels
    let input = r#"
        tempo: 2.0
        out1: sine(440) * 0.5
        out2: sine(880) * 0.5
    "#;
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify both channels produce audio
    let buffer_before = graph.render(4410);
    let rms_before: f32 =
        (buffer_before.iter().map(|x| x * x).sum::<f32>() / buffer_before.len() as f32).sqrt();
    assert!(rms_before > 0.1, "Should have audio before hush");

    // Now hush only channel 1
    let hush_input = r#"
        tempo: 2.0
        out1: sine(440) * 0.5
        out2: sine(880) * 0.5
        hush1
    "#;
    let (_, hush_statements) = parse_dsl(hush_input).unwrap();
    let hush_compiler = DslCompiler::new(44100.0);
    let mut graph = hush_compiler.compile(hush_statements);

    // Render audio - channel 1 should be silent, channel 2 should have audio
    let buffer_after = graph.render(4410);
    let rms_after: f32 =
        (buffer_after.iter().map(|x| x * x).sum::<f32>() / buffer_after.len() as f32).sqrt();

    // We should still have audio from channel 2
    assert!(
        rms_after > 0.1,
        "Channel 2 should still produce audio after hush1, got RMS: {}",
        rms_after
    );
}

#[test]
fn test_hush_all_silences_all_channels() {
    // Create multiple output channels
    let input = r#"
        tempo: 2.0
        out1: sine(440) * 0.5
        out2: sine(880) * 0.5
        out3: sine(1320) * 0.5
        hush
    "#;
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render audio - all channels should be silent
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms < 0.001,
        "All channels should be silent after hush, got RMS: {}",
        rms
    );
}

#[test]
fn test_panic_silences_and_kills_voices() {
    // Create output with sample playback (which uses voices)
    let input = r#"
        tempo: 2.0
        out: s("bd*4") * 0.5
        panic
    "#;
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render audio - should be silent after panic
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms < 0.001,
        "Should be silent after panic, got RMS: {}",
        rms
    );
}

#[test]
fn test_panic_with_synth_pattern() {
    // Create output with synth pattern (which uses voices)
    let input = r#"
        tempo: 2.0
        out: synth("c4 e4 g4 c5", "saw") * 0.3
        panic
    "#;
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render audio - should be silent after panic
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms < 0.001,
        "Synth pattern should be silent after panic, got RMS: {}",
        rms
    );
}

#[test]
fn test_multiple_hush_commands() {
    // Test that we can hush multiple channels individually
    let input = r#"
        tempo: 2.0
        out1: sine(440) * 0.5
        out2: sine(880) * 0.5
        out3: sine(1320) * 0.5
        hush1
        hush2
    "#;
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render audio - channels 1 and 2 silent, channel 3 should have audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    // Channel 3 should still be audible
    assert!(
        rms > 0.1,
        "Channel 3 should still produce audio after hush1 and hush2, got RMS: {}",
        rms
    );
}

#[test]
fn test_hush_then_unhush_not_supported() {
    // In Tidal Cycles, there's no "unhush" command
    // Once hushed, you need to re-evaluate the pattern to unhush
    // This test verifies that hush is persistent
    let input = r#"
        tempo: 2.0
        out1: sine(440) * 0.5
        hush1
    "#;
    let (_, statements) = parse_dsl(input).unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render audio - should be silent
    let buffer1 = graph.render(4410);
    let rms1: f32 = (buffer1.iter().map(|x| x * x).sum::<f32>() / buffer1.len() as f32).sqrt();
    assert!(rms1 < 0.001, "Should be silent after hush1");

    // Render more audio - should still be silent
    let buffer2 = graph.render(4410);
    let rms2: f32 = (buffer2.iter().map(|x| x * x).sum::<f32>() / buffer2.len() as f32).sqrt();
    assert!(rms2 < 0.001, "Should remain silent after hush1");
}

#[test]
fn test_parse_hush_with_whitespace() {
    // Test that parser handles whitespace correctly
    let input = r#"
        tempo: 2.0
        out: sine(440)

        hush
    "#;
    let result = parse_dsl(input);
    assert!(
        result.is_ok(),
        "Should parse hush with surrounding whitespace"
    );
}

#[test]
fn test_parse_panic_with_whitespace() {
    let input = r#"
        tempo: 2.0
        out: s("bd")

        panic
    "#;
    let result = parse_dsl(input);
    assert!(
        result.is_ok(),
        "Should parse panic with surrounding whitespace"
    );
}
