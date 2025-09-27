//! Test that $ operator works as an alias for |>

use phonon::nom_parser::{parse_expr, Expr};

#[test]
fn test_dollar_operator() {
    println!("\n=== Testing $ Operator (TidalCycles style) ===");

    let test_cases = vec![
        (r#""bd sn" $ fast 2"#, "$ with fast"),
        (r#""bd sn" $ slow 3"#, "$ with slow"),
        (r#""bd sn" $ rev"#, "$ with rev"),
        (r#""100 200" $ fast 2 $ rev"#, "chained $ operators"),
        (r#""bd*4" $ degrade"#, "$ with degrade"),
    ];

    for (code, desc) in test_cases {
        println!("\n  Testing: {} - {}", code, desc);
        match parse_expr(code) {
            Ok((rest, expr)) => {
                assert_eq!(rest, "", "Parser didn't consume all input");
                println!("    ✓ Parsed successfully");

                match expr {
                    Expr::PatternOp(_, _) => {
                        println!("    ✓ Recognized as pattern operation");
                    }
                    _ => {}
                }
            }
            Err(e) => {
                panic!("Failed to parse '{}': {:?}", code, e);
            }
        }
    }
}

#[test]
fn test_mixed_operators() {
    println!("\n=== Testing Mixed |> and $ Operators ===");

    // Both should work identically
    let equivalent_pairs = vec![
        (r#""bd sn" |> fast 2"#, r#""bd sn" $ fast 2"#),
        (r#""100 200" |> rev"#, r#""100 200" $ rev"#),
        (r#""bd" |> every 4 rev"#, r#""bd" $ every 4 rev"#),
    ];

    for (pipe_version, dollar_version) in equivalent_pairs {
        println!("\n  Comparing:");
        println!("    |> version: {}", pipe_version);
        println!("    $  version: {}", dollar_version);

        let pipe_result = parse_expr(pipe_version);
        let dollar_result = parse_expr(dollar_version);

        assert!(pipe_result.is_ok());
        assert!(dollar_result.is_ok());
        println!("    ✓ Both parse successfully");

        // Both should produce PatternOp
        match (pipe_result.unwrap().1, dollar_result.unwrap().1) {
            (Expr::PatternOp(_, _), Expr::PatternOp(_, _)) => {
                println!("    ✓ Both produce PatternOp");
            }
            _ => panic!("Results don't match"),
        }
    }
}

#[test]
fn test_tidalcycles_examples() {
    println!("\n=== Testing TidalCycles-style Examples ===");

    let examples = vec![
        r#""bd sn" $ fast 2"#,
        r#""0 3 7" $ slow 2"#,
        r#""bd sn" $ every 4 rev"#,
        r#""hh*16" $ degradeBy 0.3"#,
        r#""bd sn" $ sometimes (fast 2)"#,
        r#""100 200 300" $ fast 2 $ rev $ degrade"#,
    ];

    for code in examples {
        println!("\n  Testing: {}", code);
        match parse_expr(code) {
            Ok((rest, _)) => {
                assert_eq!(rest, "");
                println!("    ✓ TidalCycles-style syntax works!");
            }
            Err(e) => {
                panic!("Failed to parse: {:?}", e);
            }
        }
    }
}

#[test]
fn test_dollar_with_dsp_chains() {
    println!("\n=== Testing $ Operator with DSP Chains ===");

    let code = r#""bd*4" $ fast 2 >> lpf 1000 0.8"#;

    match parse_expr(code) {
        Ok((rest, expr)) => {
            assert_eq!(rest, "");
            println!("  ✓ Parsed: {}", code);

            // Should parse as: ("bd*4" $ fast 2) >> lpf
            match expr {
                Expr::Chain(left, _right) => match left.as_ref() {
                    Expr::PatternOp(_, _) => {
                        println!("  ✓ $ operator binds before >> chain");
                    }
                    other => panic!("Expected PatternOp, got: {:?}", other),
                },
                other => panic!("Expected Chain, got: {:?}", other),
            }
        }
        Err(e) => panic!("Failed to parse: {:?}", e),
    }
}
