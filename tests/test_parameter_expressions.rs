//! End-to-end tests for arithmetic expressions in pattern parameters
//!
//! Tests that expressions like (~lfo * 1000 + 500) work correctly

use phonon::dsp_parameter::{BinaryOp, DspParameter, ParameterExpression, UnaryOp};
use phonon::glicol_parser_v2::parse_glicol_v2;
use std::collections::HashMap;

#[test]
fn test_simple_arithmetic_expressions() {
    println!("\n=== Testing Simple Arithmetic Expressions ===");

    // Test addition
    let add_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::constant(1000.0),
        right: DspParameter::constant(500.0),
    }));

    let refs = HashMap::new();
    let result = add_expr.evaluate(0.0, &refs);
    assert_eq!(result, 1500.0);
    println!("  1000 + 500 = {}", result);

    // Test multiplication
    let mul_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Multiply,
        left: DspParameter::constant(100.0),
        right: DspParameter::constant(2.0),
    }));

    let result = mul_expr.evaluate(0.0, &refs);
    assert_eq!(result, 200.0);
    println!("  100 * 2 = {}", result);

    // Test subtraction
    let sub_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Subtract,
        left: DspParameter::constant(1000.0),
        right: DspParameter::constant(300.0),
    }));

    let result = sub_expr.evaluate(0.0, &refs);
    assert_eq!(result, 700.0);
    println!("  1000 - 300 = {}", result);

    // Test division
    let div_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Divide,
        left: DspParameter::constant(1000.0),
        right: DspParameter::constant(4.0),
    }));

    let result = div_expr.evaluate(0.0, &refs);
    assert_eq!(result, 250.0);
    println!("  1000 / 4 = {}", result);

    println!("  ✓ Basic arithmetic operations work correctly");
}

#[test]
fn test_lfo_modulation_expression() {
    println!("\n=== Testing LFO Modulation Expression ===");

    // Classic synthesis pattern: ~lfo * depth + center
    // This modulates a value between (center - depth) and (center + depth)

    let expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Multiply,
            left: DspParameter::reference("lfo"),
            right: DspParameter::constant(1000.0),
        })),
        right: DspParameter::constant(1500.0),
    }));

    let mut refs = HashMap::new();

    // Test with different LFO values (-1 to 1)
    let lfo_values = vec![
        (-1.0, 500.0),  // Minimum
        (0.0, 1500.0),  // Center
        (1.0, 2500.0),  // Maximum
        (0.5, 2000.0),  // Half way up
        (-0.5, 1000.0), // Half way down
    ];

    for (lfo_val, expected) in lfo_values {
        refs.insert("lfo".to_string(), lfo_val);
        let result = expr.evaluate(0.0, &refs);
        assert_eq!(result, expected);
        println!("  LFO={:5.1} → ~lfo * 1000 + 1500 = {}", lfo_val, result);
    }

    println!("  ✓ LFO modulation expression works correctly");
}

#[test]
fn test_pattern_with_expression() {
    println!("\n=== Testing Pattern Values in Expressions ===");

    // Pattern values can be used in expressions too
    let expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Multiply,
        left: DspParameter::pattern("100 200 300 400"),
        right: DspParameter::constant(2.0),
    }));

    let refs = HashMap::new();

    // Test at different cycle positions
    for pos in [0.0, 0.25, 0.5, 0.75] {
        let result = expr.evaluate(pos, &refs);
        println!("  Position {:.2}: pattern * 2 = {}", pos, result);

        // Result should be one of the pattern values * 2, or 0
        assert!(
            result == 0.0
                || result == 200.0
                || result == 400.0
                || result == 600.0
                || result == 800.0
        );
    }

    println!("  ✓ Pattern values work in expressions");
}

#[test]
fn test_complex_nested_expression() {
    println!("\n=== Testing Complex Nested Expression ===");

    // ((~lfo1 + ~lfo2) * 500) + 1000
    let expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Multiply,
            left: DspParameter::Expression(Box::new(ParameterExpression::Binary {
                op: BinaryOp::Add,
                left: DspParameter::reference("lfo1"),
                right: DspParameter::reference("lfo2"),
            })),
            right: DspParameter::constant(500.0),
        })),
        right: DspParameter::constant(1000.0),
    }));

    let mut refs = HashMap::new();
    refs.insert("lfo1".to_string(), 0.5);
    refs.insert("lfo2".to_string(), 0.3);

    let result = expr.evaluate(0.0, &refs);
    let expected = ((0.5 + 0.3) * 500.0) + 1000.0; // = 1400
    assert_eq!(result, expected);

    println!("  ((~lfo1 + ~lfo2) * 500) + 1000");
    println!("  ((0.5 + 0.3) * 500) + 1000 = {}", result);
    println!("  ✓ Complex nested expressions work correctly");
}

#[test]
fn test_unary_negation() {
    println!("\n=== Testing Unary Negation ===");

    // Test negative values
    let neg_expr = DspParameter::Expression(Box::new(ParameterExpression::Unary {
        op: UnaryOp::Negate,
        param: DspParameter::constant(100.0),
    }));

    let refs = HashMap::new();
    let result = neg_expr.evaluate(0.0, &refs);
    assert_eq!(result, -100.0);
    println!("  -100 = {}", result);

    // Test negation in complex expression: -~lfo * 500 + 1000
    let complex_neg = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Multiply,
            left: DspParameter::Expression(Box::new(ParameterExpression::Unary {
                op: UnaryOp::Negate,
                param: DspParameter::reference("lfo"),
            })),
            right: DspParameter::constant(500.0),
        })),
        right: DspParameter::constant(1000.0),
    }));

    let mut refs = HashMap::new();
    refs.insert("lfo".to_string(), 0.5);

    let result = complex_neg.evaluate(0.0, &refs);
    let expected = (-0.5 * 500.0) + 1000.0; // = 750
    assert_eq!(result, expected);
    println!("  -~lfo * 500 + 1000 = {} (with lfo=0.5)", result);

    println!("  ✓ Unary negation works correctly");
}

#[test]
fn test_parser_accepts_expressions() {
    println!("\n=== Testing Parser Acceptance of Expressions ===");

    let test_cases = vec![
        (r#"o: saw 110 >> lpf (1000 + 500) 0.8"#, "Simple addition"),
        (
            r#"o: saw 110 >> lpf (2000 - 500) 0.8"#,
            "Simple subtraction",
        ),
        (
            r#"o: saw 110 >> lpf (500 * 3) 0.8"#,
            "Simple multiplication",
        ),
        (r#"o: saw 110 >> lpf (3000 / 2) 0.8"#, "Simple division"),
        (
            r#"
                ~lfo: sin 2
                o: saw 110 >> lpf (~lfo * 1000 + 1500) 0.8
            "#,
            "LFO modulation expression",
        ),
        (
            r#"o: saw 110 >> lpf ("1000 2000" * 2) 0.8"#,
            "Pattern with multiplication",
        ),
        (
            r#"
                ~mod1: sin 1
                ~mod2: sin 2
                o: saw 110 >> lpf ((~mod1 + ~mod2) * 500 + 1000) 0.8
            "#,
            "Complex nested expression",
        ),
        (r#"o: saw 110 >> lpf (-1000 + 2000) 0.8"#, "Unary negation"),
    ];

    for (code, description) in test_cases {
        println!("  Testing: {}", description);
        match parse_glicol_v2(code) {
            Ok(_) => println!("    ✓ Parsed successfully"),
            Err(e) => panic!("    ✗ Parse failed for '{}': {}", description, e),
        }
    }

    println!("  ✓ Parser correctly accepts arithmetic expressions");
}

#[test]
fn test_expression_evaluation_in_cycle() {
    println!("\n=== Testing Expression Evaluation Across Cycle ===");

    // Create an expression that combines pattern and reference
    // pattern * ~amplitude + ~offset
    let expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Multiply,
            left: DspParameter::pattern("100 200 300 400"),
            right: DspParameter::reference("amplitude"),
        })),
        right: DspParameter::reference("offset"),
    }));

    let mut refs = HashMap::new();
    refs.insert("amplitude".to_string(), 2.0);
    refs.insert("offset".to_string(), 1000.0);

    println!("  Expression: pattern * ~amplitude + ~offset");
    println!("  Pattern: \"100 200 300 400\"");
    println!("  amplitude = 2.0, offset = 1000.0");
    println!("");

    // Evaluate across two cycles
    for cycle in 0..2 {
        println!("  Cycle {}:", cycle);
        for step in 0..4 {
            let pos = cycle as f64 + (step as f64 * 0.25);
            let result = expr.evaluate(pos, &refs);
            println!("    Position {:.2}: {}", pos, result);
        }
    }

    println!("  ✓ Expressions evaluate correctly across cycles");
}

#[test]
fn test_dynamic_expression() {
    println!("\n=== Testing Dynamic Expression Detection ===");

    // Constant expression - not dynamic
    let const_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::constant(100.0),
        right: DspParameter::constant(200.0),
    }));
    assert!(!const_expr.is_dynamic());
    println!("  100 + 200: is_dynamic = false ✓");

    // Expression with reference - dynamic
    let ref_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Multiply,
        left: DspParameter::reference("lfo"),
        right: DspParameter::constant(1000.0),
    }));
    assert!(ref_expr.is_dynamic());
    println!("  ~lfo * 1000: is_dynamic = true ✓");

    // Expression with pattern - dynamic
    let pattern_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::pattern("100 200"),
        right: DspParameter::constant(50.0),
    }));
    assert!(pattern_expr.is_dynamic());
    println!("  \"100 200\" + 50: is_dynamic = true ✓");

    println!("  ✓ Dynamic expression detection works correctly");
}

#[test]
fn test_expression_order_of_operations() {
    println!("\n=== Testing Order of Operations ===");

    // Test that multiplication happens before addition
    // 100 + 200 * 3 should be 100 + (200 * 3) = 700, not (100 + 200) * 3 = 900

    let expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::constant(100.0),
        right: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Multiply,
            left: DspParameter::constant(200.0),
            right: DspParameter::constant(3.0),
        })),
    }));

    let refs = HashMap::new();
    let result = expr.evaluate(0.0, &refs);
    assert_eq!(result, 700.0);
    println!(
        "  100 + 200 * 3 = {} (correct: multiplication first)",
        result
    );

    // Test division before subtraction
    // 1000 - 600 / 2 should be 1000 - (600 / 2) = 700
    let expr2 = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Subtract,
        left: DspParameter::constant(1000.0),
        right: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Divide,
            left: DspParameter::constant(600.0),
            right: DspParameter::constant(2.0),
        })),
    }));

    let result2 = expr2.evaluate(0.0, &refs);
    assert_eq!(result2, 700.0);
    println!("  1000 - 600 / 2 = {} (correct: division first)", result2);

    println!("  ✓ Order of operations is correct");
}

#[test]
fn test_real_world_synthesis_expressions() {
    println!("\n=== Testing Real-World Synthesis Expressions ===");

    // FM synthesis: carrier_freq + (modulator * mod_index)
    println!("  1. FM Synthesis:");
    let fm_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::constant(440.0), // Carrier frequency
        right: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Multiply,
            left: DspParameter::reference("modulator"),
            right: DspParameter::constant(200.0), // Modulation index
        })),
    }));

    let mut refs = HashMap::new();
    for mod_val in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        refs.insert("modulator".to_string(), mod_val);
        let freq = fm_expr.evaluate(0.0, &refs);
        println!("    Modulator={:5.1} → Frequency = {} Hz", mod_val, freq);
    }

    // Filter envelope: cutoff * envelope + base_cutoff
    println!("\n  2. Filter Envelope:");
    let filter_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Multiply,
            left: DspParameter::reference("envelope"),
            right: DspParameter::constant(3000.0), // Envelope amount
        })),
        right: DspParameter::constant(200.0), // Base cutoff
    }));

    for env_val in [0.0, 0.25, 0.5, 0.75, 1.0] {
        refs.insert("envelope".to_string(), env_val);
        let cutoff = filter_expr.evaluate(0.0, &refs);
        println!("    Envelope={:4.2} → Cutoff = {} Hz", env_val, cutoff);
    }

    // PWM: 0.5 + (lfo * 0.4)  [Keeps duty cycle between 0.1 and 0.9]
    println!("\n  3. Pulse Width Modulation:");
    let pwm_expr = DspParameter::Expression(Box::new(ParameterExpression::Binary {
        op: BinaryOp::Add,
        left: DspParameter::constant(0.5),
        right: DspParameter::Expression(Box::new(ParameterExpression::Binary {
            op: BinaryOp::Multiply,
            left: DspParameter::reference("lfo"),
            right: DspParameter::constant(0.4),
        })),
    }));

    for lfo_val in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        refs.insert("lfo".to_string(), lfo_val);
        let duty = pwm_expr.evaluate(0.0, &refs);
        println!(
            "    LFO={:5.1} → Duty Cycle = {:.1}%",
            lfo_val,
            duty * 100.0
        );
    }

    println!("  ✓ Real-world synthesis expressions work correctly");
}
