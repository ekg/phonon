//! Macro expansion for Phonon DSL
//!
//! This module provides compile-time macro expansion:
//! - `for i in N..M:` loops to generate multiple buses
//! - `~name[i]` indexed bus syntax
//! - `sum(~name[N..M])` to mix indexed buses
//! - Arithmetic expressions with loop variables
//!
//! The expander runs BEFORE the main parser, transforming macro constructs
//! into regular DSL code.

use regex::Regex;

/// Expand all macros in the input code
///
/// This is the main entry point. It processes:
/// 1. For loops
/// 2. Sum expressions
/// 3. Arithmetic with variables
pub fn expand_macros(input: &str) -> String {
    let mut result = input.to_string();

    // Expand for loops first (they may contain sum() calls)
    result = expand_for_loops(&result);

    // Expand sum() calls
    result = expand_sum_calls(&result);

    result
}

/// Expand for loops: `for i in N..M:` with indented body
fn expand_for_loops(input: &str) -> String {
    let mut result = String::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Check for `for VAR in START..END:`
        if let Some(captures) = parse_for_header(trimmed) {
            let var_name = captures.0;
            let start: i64 = captures.1;
            let end: i64 = captures.2;

            // Collect indented body lines
            let base_indent = get_indent(line);
            let mut body_lines = Vec::new();
            i += 1;

            while i < lines.len() {
                let body_line = lines[i];
                let body_indent = get_indent(body_line);
                let body_trimmed = body_line.trim();

                // Empty lines or more indented = part of body
                if body_trimmed.is_empty() || body_indent > base_indent {
                    body_lines.push(body_line);
                    i += 1;
                } else {
                    break;
                }
            }

            // Expand the loop
            for val in start..=end {
                for body_line in &body_lines {
                    if body_line.trim().is_empty() {
                        result.push('\n');
                        continue;
                    }

                    // Substitute variable and evaluate arithmetic
                    let expanded = substitute_and_eval(body_line, &var_name, val);
                    // Remove extra indentation from body
                    let dedented = dedent(body_line, base_indent + 4);
                    let expanded_dedented = substitute_and_eval(&dedented, &var_name, val);
                    result.push_str(&expanded_dedented);
                    result.push('\n');
                }
            }
        } else {
            result.push_str(line);
            result.push('\n');
            i += 1;
        }
    }

    result
}

/// Parse a for loop header: `for i in 1..10:`
/// Returns (variable_name, start, end) or None
fn parse_for_header(line: &str) -> Option<(String, i64, i64)> {
    let re = Regex::new(r"^for\s+(\w+)\s+in\s+(-?\d+)\.\.(-?\d+)\s*:?\s*$").unwrap();

    if let Some(caps) = re.captures(line) {
        let var_name = caps.get(1)?.as_str().to_string();
        let start: i64 = caps.get(2)?.as_str().parse().ok()?;
        let end: i64 = caps.get(3)?.as_str().parse().ok()?;
        Some((var_name, start, end))
    } else {
        None
    }
}

/// Get the indentation level of a line (number of leading spaces)
fn get_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Remove `amount` spaces of indentation
fn dedent(line: &str, amount: usize) -> String {
    let current_indent = get_indent(line);
    if current_indent >= amount {
        line[amount..].to_string()
    } else {
        line.trim_start().to_string()
    }
}

/// Substitute variable and evaluate arithmetic expressions
fn substitute_and_eval(line: &str, var_name: &str, value: i64) -> String {
    let mut result = line.to_string();

    // First, substitute ~name[var] with ~nameN
    let indexed_bus_re = Regex::new(&format!(r"~(\w+)\[{}\]", var_name)).unwrap();
    result = indexed_bus_re
        .replace_all(&result, |caps: &regex::Captures| {
            format!("~{}{}", &caps[1], value)
        })
        .to_string();

    // Evaluate arithmetic expressions containing the variable
    result = eval_arithmetic(&result, var_name, value);

    result
}

/// Evaluate arithmetic expressions containing a variable
fn eval_arithmetic(input: &str, var_name: &str, value: i64) -> String {
    let mut result = input.to_string();

    // Pattern: (expr * var), (var * expr), (expr + var), etc.
    // We need to find and evaluate these expressions

    // Handle parenthesized expressions first: (110 * i), (i + 55), etc.
    let paren_re = Regex::new(r"\(([^()]+)\)").unwrap();

    // Iterate until no more changes (for nested parens)
    loop {
        let new_result = paren_re
            .replace_all(&result, |caps: &regex::Captures| {
                let inner = &caps[1];
                if inner.contains(var_name) {
                    // Try to evaluate this expression
                    if let Some(val) = try_eval_simple_expr(inner, var_name, value) {
                        format_number(val)
                    } else {
                        // Can't evaluate, keep as-is but substitute variable
                        format!("({})", inner.replace(var_name, &value.to_string()))
                    }
                } else {
                    // No variable - try to evaluate as pure numeric expression
                    if let Some(val) = try_eval_pure_numeric(inner) {
                        format_number(val)
                    } else {
                        // Can't evaluate, keep parentheses
                        caps[0].to_string()
                    }
                }
            })
            .to_string();

        if new_result == result {
            break;
        }
        result = new_result;
    }

    // Handle non-parenthesized simple expressions: 110 * i, i + 55
    // Look for patterns like: NUMBER OP VAR or VAR OP NUMBER
    // Handle simple patterns: number OP var, var OP number
    // number * var
    let re = Regex::new(&format!(r"(\d+(?:\.\d+)?)\s*\*\s*{}", var_name)).unwrap();
    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let num: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
            format_number(num * value as f64)
        })
        .to_string();

    // var * number
    let re = Regex::new(&format!(r"{}\s*\*\s*(\d+(?:\.\d+)?)", var_name)).unwrap();
    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let num: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
            format_number(value as f64 * num)
        })
        .to_string();

    // number + var
    let re = Regex::new(&format!(r"(\d+(?:\.\d+)?)\s*\+\s*{}", var_name)).unwrap();
    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let num: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
            format_number(num + value as f64)
        })
        .to_string();

    // var + number
    let re = Regex::new(&format!(r"{}\s*\+\s*(\d+(?:\.\d+)?)", var_name)).unwrap();
    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let num: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
            format_number(value as f64 + num)
        })
        .to_string();

    // number / var
    let re = Regex::new(&format!(r"(\d+(?:\.\d+)?)\s*/\s*{}", var_name)).unwrap();
    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let num: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
            if value != 0 {
                format_number(num / value as f64)
            } else {
                "inf".to_string()
            }
        })
        .to_string();

    // var / number
    let re = Regex::new(&format!(r"{}\s*/\s*(\d+(?:\.\d+)?)", var_name)).unwrap();
    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let num: f64 = caps.get(1).unwrap().as_str().parse().unwrap();
            if num != 0.0 {
                format_number(value as f64 / num)
            } else {
                "inf".to_string()
            }
        })
        .to_string();

    // Finally, replace any remaining bare variable with its value
    let bare_var_re = Regex::new(&format!(r"\b{}\b", var_name)).unwrap();
    result = bare_var_re.replace_all(&result, &value.to_string()).to_string();

    result
}

/// Try to evaluate a simple arithmetic expression
fn try_eval_simple_expr(expr: &str, var_name: &str, value: i64) -> Option<f64> {
    // Substitute variable first
    let substituted = expr.replace(var_name, &format!("{}.0", value));

    // Try to parse as simple binary expression: A op B
    // Check each operator in order of precedence
    for op_char in ['*', '/', '+', '-'] {
        if let Some(pos) = substituted.find(op_char) {
            let left = substituted[..pos].trim();
            let right = substituted[pos + 1..].trim();

            if let (Ok(a), Ok(b)) = (left.parse::<f64>(), right.parse::<f64>()) {
                let result = match op_char {
                    '*' => a * b,
                    '/' => a / b,
                    '+' => a + b,
                    '-' => a - b,
                    _ => return None,
                };
                return Some(result);
            }
        }
    }

    // Try parsing as just a number (after substitution)
    substituted.trim().parse::<f64>().ok()
}

/// Try to evaluate a pure numeric expression (no variables)
/// Handles operator precedence: * and / before + and -
fn try_eval_pure_numeric(expr: &str) -> Option<f64> {
    let trimmed = expr.trim();

    // First, try to parse as just a number
    if let Ok(n) = trimmed.parse::<f64>() {
        return Some(n);
    }

    // Tokenize: split into numbers and operators
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();

    for ch in trimmed.chars() {
        if ch == '+' || ch == '-' || ch == '*' || ch == '/' {
            if !current.is_empty() {
                tokens.push(current.trim().to_string());
                current = String::new();
            } else if ch == '-' && tokens.is_empty() {
                // Negative number at start
                current.push(ch);
                continue;
            }
            tokens.push(ch.to_string());
        } else if ch.is_whitespace() {
            // Skip whitespace
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current.trim().to_string());
    }

    if tokens.is_empty() {
        return None;
    }

    // Parse tokens into numbers and operators
    let mut values: Vec<f64> = Vec::new();
    let mut ops: Vec<char> = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        if i % 2 == 0 {
            // Expect a number
            if let Ok(n) = tokens[i].parse::<f64>() {
                values.push(n);
            } else {
                return None; // Not a valid expression
            }
        } else {
            // Expect an operator
            if tokens[i].len() == 1 {
                ops.push(tokens[i].chars().next().unwrap());
            } else {
                return None;
            }
        }
        i += 1;
    }

    // First pass: handle * and /
    let mut new_values: Vec<f64> = Vec::new();
    let mut new_ops: Vec<char> = Vec::new();
    new_values.push(values[0]);

    for i in 0..ops.len() {
        let op = ops[i];
        let right = values[i + 1];

        if op == '*' || op == '/' {
            let left = new_values.pop().unwrap();
            let result = if op == '*' { left * right } else { left / right };
            new_values.push(result);
        } else {
            new_ops.push(op);
            new_values.push(right);
        }
    }

    // Second pass: handle + and -
    let mut result = new_values[0];
    for i in 0..new_ops.len() {
        let op = new_ops[i];
        let right = new_values[i + 1];
        result = if op == '+' { result + right } else { result - right };
    }

    Some(result)
}

/// Format a number nicely (integer if whole, otherwise float)
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e10 {
        format!("{}", n as i64)
    } else {
        // Round to reasonable precision
        let rounded = (n * 1000.0).round() / 1000.0;
        if rounded.fract() == 0.0 {
            format!("{}", rounded as i64)
        } else {
            format!("{}", rounded)
        }
    }
}

/// Expand sum() calls: sum(~name[N..M]) -> (~nameN + ~nameN+1 + ... + ~nameM)
fn expand_sum_calls(input: &str) -> String {
    let sum_re = Regex::new(r"sum\(~(\w+)\[(\d+)\.\.(\d+)\]\)").unwrap();

    sum_re
        .replace_all(input, |caps: &regex::Captures| {
            let name = &caps[1];
            let start: i64 = caps[2].parse().unwrap();
            let end: i64 = caps[3].parse().unwrap();

            let terms: Vec<String> = (start..=end).map(|i| format!("~{}{}", name, i)).collect();

            if terms.is_empty() {
                "0".to_string()
            } else {
                format!("({})", terms.join(" + "))
            }
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_for_header() {
        assert_eq!(
            parse_for_header("for i in 1..10:"),
            Some(("i".to_string(), 1, 10))
        );
        assert_eq!(
            parse_for_header("for x in 0..5:"),
            Some(("x".to_string(), 0, 5))
        );
        assert_eq!(parse_for_header("not a for loop"), None);
    }

    #[test]
    fn test_expand_sum() {
        assert_eq!(
            expand_sum_calls("sum(~osc[1..3])"),
            "(~osc1 + ~osc2 + ~osc3)"
        );
        assert_eq!(
            expand_sum_calls("out $ sum(~s[0..2]) * 0.5"),
            "out $ (~s0 + ~s1 + ~s2) * 0.5"
        );
    }

    #[test]
    fn test_eval_arithmetic() {
        assert_eq!(eval_arithmetic("(110 * i)", "i", 2), "220");
        assert_eq!(eval_arithmetic("(i + 55)", "i", 100), "155");
        assert_eq!(eval_arithmetic("sine (110 * i)", "i", 3), "sine 330");
    }

    #[test]
    fn test_indexed_bus_substitution() {
        assert_eq!(
            substitute_and_eval("~osc[i] $ sine 440", "i", 5),
            "~osc5 $ sine 440"
        );
    }

    #[test]
    fn test_full_expansion() {
        let code = r#"
for i in 1..2:
    ~s[i] $ sine (110 * i)
out $ sum(~s[1..2])
"#;
        let expanded = expand_macros(code);
        assert!(expanded.contains("~s1 $ sine 110"));
        assert!(expanded.contains("~s2 $ sine 220"));
        assert!(expanded.contains("(~s1 + ~s2)"));
    }
}
