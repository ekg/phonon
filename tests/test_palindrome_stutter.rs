use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_palindrome_transform() {
    let input = r#"
        cps: 2.0
        out $ s("bd sn" $ palindrome) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 2 seconds (4 cycles at 2 CPS)
    let buffer = graph.render(88200);

    // Calculate RMS
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("Palindrome RMS: {}", rms);
    assert!(
        rms > 0.0001,
        "Palindrome should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_stutter_transform() {
    let input = r#"
        cps: 2.0
        out $ s("bd sn" $ stutter 3) * 0.5
    "#;

    let (_, statements) = parse_dsl(input).expect("Should parse DSL");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 second (2 cycles at 2 CPS)
    let buffer = graph.render(44100);

    // Calculate RMS
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("Stutter RMS: {}", rms);
    assert!(
        rms > 0.0001,
        "Stutter should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_palindrome_parsing() {
    let input = r#"
        cps: 2.0
        out $ "1 2 3" $ palindrome
    "#;

    let result = parse_dsl(input);
    println!("Parse result: {:?}", result);
    assert!(result.is_ok(), "Should parse palindrome transform");
}

#[test]
fn test_stutter_parsing() {
    let input = r#"
        cps: 2.0
        out $ "1 2 3" $ stutter 4
    "#;

    let result = parse_dsl(input);
    println!("Parse result: {:?}", result);
    assert!(result.is_ok(), "Should parse stutter transform");
}
