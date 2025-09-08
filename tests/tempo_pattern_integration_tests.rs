use phonon::phonon_lang::{PhononEnv, PhononParser};
use phonon::pattern_lang_parser::{PatternParser, PatternExpr, TransformOp};

#[test]
fn test_bpm_setting() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        bpm: 120
        ~drums: s "bd sn"
        o: ~drums
    "#;
    
    env.eval(code).expect("Failed to parse");
    assert_eq!(env.cps, 0.5); // 120 BPM = 0.5 CPS
}

#[test]
fn test_cps_setting() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        cps: 2.0
        ~pattern: s "bd*4"
        o: ~pattern
    "#;
    
    env.eval(code).expect("Failed to parse");
    assert_eq!(env.cps, 2.0);
}

#[test]
fn test_cps_overrides_bpm() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        bpm: 120
        cps: 1.0
        o: s "bd"
    "#;
    
    env.eval(code).expect("Failed to parse");
    assert_eq!(env.cps, 1.0); // CPS should override BPM
}

#[test]
fn test_pattern_transformation_fast() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        ~drums: s "bd sn" >> fast 2
        o: ~drums
    "#;
    
    env.eval(code).expect("Failed to parse with fast transformation");
}

#[test]
fn test_pattern_transformation_chain() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        ~drums: s "bd sn hh cp" >> fast 2 >> rev
        o: ~drums
    "#;
    
    env.eval(code).expect("Failed to parse transformation chain");
}

#[test]
fn test_pattern_every_transformation() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        ~drums: s "bd sn" >> every 4 (rev)
        o: ~drums
    "#;
    
    env.eval(code).expect("Failed to parse every transformation");
}

#[test]
fn test_multiple_pattern_definitions() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        bpm: 128
        ~kick: s "bd*4"
        ~hats: s "hh*8" >> degrade
        ~snare: s "~ sn ~ sn"
        o: ~kick
    "#;
    
    env.eval(code).expect("Failed to parse multiple patterns");
    assert_eq!(env.cps, 128.0 / 240.0); // 128 BPM conversion
}

#[test]
fn test_pattern_palindrome() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        ~melody: s "c4 d4 e4 f4" >> palindrome
        o: ~melody
    "#;
    
    env.eval(code).expect("Failed to parse palindrome");
}

#[test]
fn test_complex_transformation_chain() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        bpm: 140
        ~beat: s "bd cp sn cp" >> fast 2 >> every 8 (rev) >> degrade
        o: ~beat
    "#;
    
    env.eval(code).expect("Failed to parse complex chain");
}

#[test]
fn test_pattern_slow() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        ~bass: s "c2 e2 g2 c3" >> slow 2
        o: ~bass
    "#;
    
    env.eval(code).expect("Failed to parse slow transformation");
}

#[test]
fn test_degradeby() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        ~hats: s "hh*16" >> degradeBy 0.3
        o: ~hats
    "#;
    
    env.eval(code).expect("Failed to parse degradeBy");
}

#[test]
fn test_output_short_form() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        ~drums: s "bd sn"
        o: ~drums
    "#;
    
    env.eval(code).expect("Failed to parse with o: output");
}

#[test]
fn test_output_long_form() {
    let mut env = PhononEnv::new(44100.0);
    
    let code = r#"
        ~drums: s "bd sn"
        out: ~drums
    "#;
    
    env.eval(code).expect("Failed to parse with out: output");
}

#[test]
fn test_bpm_conversion_values() {
    let mut env = PhononEnv::new(44100.0);
    
    // Test various BPM values
    let test_cases = vec![
        (60.0, 0.25),   // 60 BPM = 0.25 CPS
        (120.0, 0.5),   // 120 BPM = 0.5 CPS
        (128.0, 128.0/240.0), // House tempo
        (140.0, 140.0/240.0), // Dubstep
        (174.0, 174.0/240.0), // DnB
    ];
    
    for (bpm, expected_cps) in test_cases {
        let code = format!("bpm: {}\no: s \"bd\"", bpm);
        env.eval(&code).expect("Failed to parse");
        assert!((env.cps - expected_cps).abs() < 0.001, 
                "BPM {} should give CPS {}, got {}", bpm, expected_cps, env.cps);
    }
}

#[test]
fn test_pattern_parser_integration() {
    // Test the pattern parser directly
    let test_cases = vec![
        r#"s "bd sn""#,
        r#"s "bd sn" >> fast 2"#,
        r#"s "bd sn" >> rev"#,
        r#"s "bd sn" >> fast 2 >> rev"#,
        r#"s "bd sn" >> every 4 (slow 2)"#,
    ];
    
    for pattern in test_cases {
        let mut parser = PatternParser::new(pattern);
        parser.parse().expect(&format!("Failed to parse: {}", pattern));
    }
}

#[test]
fn test_pattern_parser_transform_ops() {
    let pattern = r#"s "bd sn" >> fast 2"#;
    let mut parser = PatternParser::new(pattern);
    let expr = parser.parse().expect("Failed to parse");
    
    match expr {
        PatternExpr::Transform { op, .. } => {
            match op {
                TransformOp::Fast(n) => assert_eq!(n, 2.0),
                _ => panic!("Expected Fast transform"),
            }
        },
        _ => panic!("Expected Transform expression"),
    }
}

#[test]
fn test_pattern_parser_chained_transforms() {
    let pattern = r#"s "bd sn" >> fast 2 >> rev >> degrade"#;
    let mut parser = PatternParser::new(pattern);
    let expr = parser.parse().expect("Failed to parse");
    
    // Should be nested transforms, outermost is degrade
    match expr {
        PatternExpr::Transform { op, .. } => {
            assert!(matches!(op, TransformOp::Degrade));
        },
        _ => panic!("Expected Transform expression"),
    }
}