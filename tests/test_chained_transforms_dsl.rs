/// Test that verifies chained transforms work correctly in the DSL
/// This addresses the bug where s("bd sn" $ fast 2 $ rev) would produce no audio
/// because the parser only extracted one level of transforms
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
#[ignore] // Uses old unified_graph_parser; superseded by compositional parser tests
fn test_single_transform_in_dsl() {
    // Baseline: single transform should work
    let input = r#"
        cps: 1.0
        out $ s("bd sn" $ fast 2) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Single transform should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
#[ignore] // Uses old unified_graph_parser; superseded by compositional parser tests
fn test_double_chained_transforms_in_dsl() {
    // The bug: this would produce no audio before the fix
    let input = r#"
        cps: 1.0
        out $ s("bd sn" $ fast 2 $ rev) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
#[ignore] // Uses old unified_graph_parser; superseded by compositional parser tests
fn test_triple_chained_transforms_in_dsl() {
    // Even more complex: three transforms chained
    let input = r#"
        cps: 1.0
        out $ s("bd sn hh" $ fast 2 $ rev $ slow 0.5) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(88200); // 2 seconds

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Triple chained transforms should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
#[ignore] // Uses old unified_graph_parser; superseded by compositional parser tests
fn test_chained_transforms_with_dsp_params() {
    // Verify that DSP parameters (gain, pan, etc.) work with chained transforms
    let input = r#"
        cps: 1.0
        out $ s("bd sn" $ fast 2 $ rev, 0.8, 0.5) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    // Should produce audible output
    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Chained transforms with DSP params should produce audio, got RMS {:.6}",
        rms
    );
}

#[test]
#[ignore] // Uses old unified_graph_parser; superseded by compositional parser tests
fn test_different_transform_combinations() {
    // Test various combinations to ensure the fix is robust
    let test_cases = vec![
        ("s(\"bd sn\" $ degrade $ fast 2)", "degrade + fast"),
        (
            "s(\"bd sn hh cp\" $ palindrome $ slow 0.5)",
            "palindrome + slow",
        ),
        ("s(\"bd\" $ stutter 2 $ rev)", "stutter + rev"),
    ];

    for (pattern, description) in test_cases {
        let input = format!(
            r#"
            cps: 1.0
            out $ {} * 0.5
        "#,
            pattern
        );

        let (_, statements) =
            parse_dsl(&input).unwrap_or_else(|_| panic!("Should parse pattern: {}", description));

        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);
        let audio = graph.render(88200); // 2 seconds

        let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
        assert!(
            rms > 0.0003,
            "Transform combination '{}' should produce audio, got RMS {:.6}",
            description,
            rms
        );
    }
}
