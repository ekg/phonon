/// Test BPM setting and conversion
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
#[ignore = "uses DslCompiler path which has pre-existing issues with sample loading"]
fn test_bpm_120_equals_cps_2() {
    // bpm 120 should equal cps 2.0 (120 / 60 = 2)
    let input_bpm = r#"
        bpm 120
        out $ s "bd*4" * 0.5
    "#;

    let input_cps = r#"
        cps: 2.0
        out $ s "bd*4" * 0.5
    "#;

    // Parse both
    let (_, statements_bpm) = parse_dsl(input_bpm).expect("Should parse BPM");
    let (_, statements_cps) = parse_dsl(input_cps).expect("Should parse CPS");

    // Compile both
    let compiler_bpm = DslCompiler::new(44100.0);
    let mut graph_bpm = compiler_bpm.compile(statements_bpm);

    let compiler_cps = DslCompiler::new(44100.0);
    let mut graph_cps = compiler_cps.compile(statements_cps);

    // Render 1 second of audio
    let audio_bpm = graph_bpm.render(44100);
    let audio_cps = graph_cps.render(44100);

    // Both should produce audio
    let rms_bpm: f32 =
        (audio_bpm.iter().map(|x| x * x).sum::<f32>() / audio_bpm.len() as f32).sqrt();
    let rms_cps: f32 =
        (audio_cps.iter().map(|x| x * x).sum::<f32>() / audio_cps.len() as f32).sqrt();

    assert!(
        rms_bpm > 0.0003,
        "BPM should produce audio, got RMS {:.6}",
        rms_bpm
    );
    assert!(
        rms_cps > 0.0003,
        "CPS should produce audio, got RMS {:.6}",
        rms_cps
    );

    // RMS should be similar (within 10%)
    let ratio = rms_bpm / rms_cps;
    assert!(
        ratio > 0.9 && ratio < 1.1,
        "BPM and CPS should produce similar audio levels. BPM RMS: {:.6}, CPS RMS: {:.6}, ratio: {:.2}",
        rms_bpm, rms_cps, ratio
    );
}

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_various_bpm_values() {
    // Test common BPM values
    let test_cases = vec![
        (60.0, 1.0),  // 60 BPM = 1 CPS
        (120.0, 2.0), // 120 BPM = 2 CPS
        (90.0, 1.5),  // 90 BPM = 1.5 CPS
        (180.0, 3.0), // 180 BPM = 3 CPS
    ];

    for (bpm, expected_cps) in test_cases {
        let input = format!(
            r#"
            bpm {}
            out $ s "bd sn" * 0.5
        "#,
            bpm
        );

        let (_, statements) =
            parse_dsl(&input).unwrap_or_else(|_| panic!("Should parse BPM {}", bpm));

        let compiler = DslCompiler::new(44100.0);
        let mut graph = compiler.compile(statements);

        // Render a bit
        let audio = graph.render(4410); // 0.1 seconds

        let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
        assert!(
            rms > 0.0003,
            "BPM {} (= {} CPS) should produce audio, got RMS {:.6}",
            bpm,
            expected_cps,
            rms
        );
    }
}

#[test]
#[ignore = "uses DslCompiler path which has pre-existing issues with sample loading"]
fn test_bpm_without_colon() {
    // bpm should work without colon (unlike cps/tempo which require colon)
    let input = r#"
        bpm 120
        out $ s "bd sn hh cp" * 0.5
    "#;

    let result = parse_dsl(input);
    assert!(result.is_ok(), "BPM should parse without colon");

    let (_, statements) = result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.001, "BPM should produce audio");
}

#[test]
#[ignore = "requires sample files (dirt-samples) to be installed"]
fn test_tempo_alias_still_works() {
    // Make sure tempo: still works as alias for cps:
    let input = r#"
        tempo: 0.5
        out $ s "bd sn" * 0.5
    "#;

    let result = parse_dsl(input);
    assert!(result.is_ok(), "tempo: should still work as alias for cps:");

    let (_, statements) = result.unwrap();
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);
    let audio = graph.render(44100);

    let rms: f32 = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(rms > 0.001, "tempo: should produce audio");
}
