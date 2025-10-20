use phonon::compositional_parser::*;

fn main() {
    // Test 1: Function call without parens
    let test1 = "lpf 500 0.8";
    println!("Test 1: {}", test1);
    match parse_function_call(test1) {
        Ok((rest, expr)) => println!("✓ Parsed: {:?}, rest: '{}'", expr, rest),
        Err(e) => println!("✗ Error: {:?}", e),
    }
    println!();

    // Test 2: Bus assignment
    let test2 = "~drums: s \"bd sn hh cp\"";
    println!("Test 2: {}", test2);
    match parse_statement(test2) {
        Ok((rest, stmt)) => println!("✓ Parsed: {:?}, rest: '{}'", stmt, rest),
        Err(e) => println!("✗ Error: {:?}", e),
    }
    println!();

    // Test 3: Chain
    let test3 = "s \"bd\" # lpf 500 0.8";
    println!("Test 3: {}", test3);
    match parse_expr(test3) {
        Ok((rest, expr)) => println!("✓ Parsed: {:?}, rest: '{}'", expr, rest),
        Err(e) => println!("✗ Error: {:?}", e),
    }
    println!();

    // Test 4: Bus ref in parameter
    let test4 = "s \"hh\" # lpf ~cutoffs 0.8";
    println!("Test 4: {}", test4);
    match parse_expr(test4) {
        Ok((rest, expr)) => println!("✓ Parsed: {:?}, rest: '{}'", expr, rest),
        Err(e) => println!("✗ Error: {:?}", e),
    }
}
