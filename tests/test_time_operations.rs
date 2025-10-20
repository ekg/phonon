/// Test time-shifting operations: late, early, dup
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_late_transform() {
    // late should shift pattern forward in time
    let input = r#"
        cps: 1.0
        out: s("bd sn" $ late 0.25) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Late transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_early_transform() {
    // early should shift pattern backward in time
    let input = r#"
        cps: 1.0
        out: s("bd sn" $ early 0.25) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Early transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_dup_transform() {
    // dup should repeat pattern n times within one cycle
    let input_normal = r#"
        cps: 1.0
        out: s "bd sn" * 0.5
    "#;

    let input_dup = r#"
        cps: 1.0
        out: s("bd sn" $ dup 3) * 0.5
    "#;

    // Render normal pattern
    let (_, statements) = parse_dsl(input_normal).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio_normal = graph.render(44100);
    let rms_normal: f32 =
        (audio_normal.iter().map(|x| x * x).sum::<f32>() / audio_normal.len() as f32).sqrt();

    // Render dup pattern
    let (_, statements) = parse_dsl(input_dup).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio_dup = graph.render(44100);
    let rms_dup: f32 =
        (audio_dup.iter().map(|x| x * x).sum::<f32>() / audio_dup.len() as f32).sqrt();

    println!("Normal RMS: {:.6}", rms_normal);
    println!("Dup(3) RMS: {:.6}", rms_dup);

    // Both should produce audio
    assert!(rms_normal > 0.001, "Normal pattern should produce audio");
    assert!(
        rms_dup > 0.001,
        "Dup transform should produce audio, got RMS {:.6}",
        rms_dup
    );
}

#[test]
fn test_late_with_chained_transforms() {
    // late should work with other transforms
    let input = r#"
        cps: 1.0
        out: s("bd sn" $ fast 2 $ late 0.125) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Late with chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
fn test_early_with_chained_transforms() {
    // early should work with other transforms
    let input = r#"
        cps: 1.0
        out: s("bd sn" $ fast 2 $ early 0.125) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Early with chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}
