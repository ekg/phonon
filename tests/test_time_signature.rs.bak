/// Test time signature support in BPM
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_bpm_with_time_signature_4_4() {
    // bpm 120 [4/4] should work
    let input = r#"
        bpm 120 [4/4]
        out: s("bd sn hh cp") * 0.5
    "#;

    let result = parse_dsl(input);
    assert!(result.is_ok(), "Should parse bpm with [4/4] time signature");

    let (_, statements) = result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.0003, "Should produce audio");
}

#[test]
fn test_bpm_with_time_signature_3_4() {
    // bpm 120 [3/4] (waltz time)
    let input = r#"
        bpm 120 [3/4]
        out: s("bd sn hh") * 0.5
    "#;

    let result = parse_dsl(input);
    assert!(result.is_ok(), "Should parse bpm with [3/4] time signature");

    let (_, statements) = result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.0003, "Should produce audio");
}

#[test]
fn test_bpm_with_time_signature_6_8() {
    // bpm 120 [6/8] (compound time)
    let input = r#"
        bpm 120 [6/8]
        out: s("bd sn hh cp sn hh") * 0.5
    "#;

    let result = parse_dsl(input);
    assert!(result.is_ok(), "Should parse bpm with [6/8] time signature");

    let (_, statements) = result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.0003, "Should produce audio");
}

#[test]
fn test_bpm_without_time_signature_defaults_4_4() {
    // bpm 120 (without brackets) should default to 4/4
    let input_without = r#"
        bpm 120
        out: s("bd sn hh cp") * 0.5
    "#;

    let input_with = r#"
        bpm 120 [4/4]
        out: s("bd sn hh cp") * 0.5
    "#;

    let (_, statements1) = parse_dsl(input_without).unwrap();
    let (_, statements2) = parse_dsl(input_with).unwrap();

    let compiler1 = DslCompiler::new(44100.0);
    let mut graph1 = compiler1.compile(statements1);
    let audio1 = graph1.render(44100);

    let compiler2 = DslCompiler::new(44100.0);
    let mut graph2 = compiler2.compile(statements2);
    let audio2 = graph2.render(44100);

    let rms1: f32 = (audio1.iter().map(|x| x * x).sum::<f32>() / audio1.len() as f32).sqrt();
    let rms2: f32 = (audio2.iter().map(|x| x * x).sum::<f32>() / audio2.len() as f32).sqrt();

    // Both should produce similar audio (within 10%)
    let ratio = rms1 / rms2;
    assert!(
        ratio > 0.9 && ratio < 1.1,
        "With and without [4/4] should produce similar results. Ratio: {:.2}",
        ratio
    );
}
