#![allow(unused_assignments, unused_mut)]
//! TidalCycles-style pattern display

use crate::pattern::{Pattern, State, TimeSpan, Fraction};
use std::collections::HashMap;

/// Display a pattern in TidalCycles format
pub fn display_pattern<T: std::fmt::Display + Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycles: usize,
) -> String {
    let mut output = String::new();
    
    for cycle in 0..cycles {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let span = TimeSpan::new(begin, end);
        let state = State {
            span,
            controls: HashMap::new(),
        };
        
        let haps = pattern.query(&state);
        
        // Group events by their start time
        let mut events_by_time: Vec<(f64, f64, String)> = Vec::new();
        
        for hap in &haps {
            let start = hap.part.begin.to_float();
            let end = hap.part.end.to_float();
            let value = format!("{}", hap.value);
            events_by_time.push((start, end, value));
        }
        
        // Sort by start time
        events_by_time.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // Format as TidalCycles-style output
        for (start, end, value) in events_by_time {
            // Show as fractions relative to cycle
            let cycle_start = start - cycle as f64;
            let cycle_end = end - cycle as f64;
            
            // Convert to simple fractions
            let start_frac = to_simple_fraction(cycle_start);
            let end_frac = to_simple_fraction(cycle_end);
            
            output.push_str(&format!("({} -> {}): {}\n", start_frac, end_frac, value));
        }
    }
    
    output
}

/// Convert a float to a simple fraction string
fn to_simple_fraction(f: f64) -> String {
    // Common fractions in music
    let denominators = [1, 2, 3, 4, 6, 8, 12, 16];
    
    for &denom in &denominators {
        let num = (f * denom as f64).round() as i32;
        if ((num as f64 / denom as f64) - f).abs() < 0.001 {
            if num == 0 {
                return "0".to_string();
            } else if denom == 1 {
                return format!("{}", num);
            } else {
                return format!("{}/{}", num, denom);
            }
        }
    }
    
    // Fall back to decimal
    format!("{:.3}", f)
}

/// Print pattern info in a nice format
pub fn print_pattern(pattern_str: &str) {
    use crate::mini_notation::parse_mini_notation;
    
    println!("\"{}\"", pattern_str);
    
    let pattern = parse_mini_notation(pattern_str);
    let display = display_pattern(&pattern, 1);
    
    print!("{}", display);
}