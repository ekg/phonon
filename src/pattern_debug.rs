#![allow(unused_assignments, unused_mut)]
//! Pattern debugging utilities for visualizing and testing patterns

use crate::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;

/// Render a pattern as ASCII art for debugging
pub fn pattern_to_ascii<T: std::fmt::Display + Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycles: usize,
    resolution: usize,
) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "Pattern visualization ({cycles} cycles, {resolution} steps/cycle):\n"
    ));
    output.push_str(&"-".repeat(resolution * cycles + cycles - 1));
    output.push('\n');

    // Query pattern for each cycle
    for cycle in 0..cycles {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let span = TimeSpan::new(begin, end);
        let state = State {
            span: span,
            controls: HashMap::new(),
        };

        let haps = pattern.query(&state);

        // Create a grid for this cycle
        let mut grid = vec!['.'; resolution];

        for hap in &haps {
            let start_pos =
                ((hap.part.begin.to_float() - cycle as f64) * resolution as f64) as usize;
            let end_pos = ((hap.part.end.to_float() - cycle as f64) * resolution as f64) as usize;

            // Get first char of the value for display
            let display_char = format!("{}", hap.value).chars().next().unwrap_or('?');

            for i in start_pos..end_pos.min(resolution) {
                if i < resolution {
                    grid[i] = display_char;
                }
            }
        }

        // Print cycle number and grid
        output.push_str(&format!("{cycle}:"));
        for ch in grid {
            output.push(ch);
        }

        if cycle < cycles - 1 {
            output.push('|');
        }
        output.push('\n');
    }

    output
}

/// Describe pattern events in text format
pub fn describe_pattern<T: std::fmt::Display + Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
    cycles: usize,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("Pattern events for {cycles} cycle(s):\n"));

    for cycle in 0..cycles {
        let begin = Fraction::new(cycle as i64, 1);
        let end = Fraction::new((cycle + 1) as i64, 1);
        let span = TimeSpan::new(begin, end);
        let state = State {
            span: span,
            controls: HashMap::new(),
        };

        let haps = pattern.query(&state);

        output.push_str(&format!("Cycle {cycle}:\n"));

        if haps.is_empty() {
            output.push_str("  (silence)\n");
        } else {
            for hap in &haps {
                let start = hap.part.begin.to_float();
                let end = hap.part.end.to_float();
                let duration = end - start;
                output.push_str(&format!(
                    "  [{:.3} - {:.3}] ({:.3}s): {}\n",
                    start, end, duration, hap.value
                ));
            }
        }
    }

    output
}

/// Test if patterns are playing simultaneously (stacked)
pub fn verify_polyphony<T: std::fmt::Display + Clone + Send + Sync + 'static>(
    pattern: &Pattern<T>,
) -> bool {
    let span = TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1));
    let state = State {
        span,
        controls: HashMap::new(),
    };

    let haps = pattern.query(&state);

    // Check if any events overlap
    for i in 0..haps.len() {
        for j in i + 1..haps.len() {
            let a = &haps[i];
            let b = &haps[j];

            // Check if they overlap
            if a.part.begin.to_float() < b.part.end.to_float()
                && b.part.begin.to_float() < a.part.end.to_float()
            {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_notation::parse_mini_notation;

    #[test]
    fn test_pattern_visualization() {
        let pattern = parse_mini_notation("bd sn hh cp");
        let ascii = pattern_to_ascii(&pattern, 2, 16);
        println!("{}", ascii);
        assert!(ascii.contains("bbbb"));
        assert!(ascii.contains("ssss"));
    }

    #[test]
    fn test_pattern_description() {
        let pattern = parse_mini_notation("bd sn");
        let desc = describe_pattern(&pattern, 1);
        println!("{}", desc);
        assert!(desc.contains("bd"));
        assert!(desc.contains("sn"));
    }

    #[test]
    fn test_polyphony_detection() {
        use crate::pattern::Pattern;

        // Sequential pattern - no polyphony
        let seq = parse_mini_notation("bd sn hh cp");
        assert!(!verify_polyphony(&seq));

        // Create a truly polyphonic pattern using stack()
        // These patterns will overlap at positions 0-0.5 and 0.5-1.0
        let p1 = parse_mini_notation("bd bd");
        let p2 = parse_mini_notation("hh hh");
        let poly = Pattern::stack(vec![p1, p2]);

        let span = TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1));
        let state = State {
            span,
            controls: HashMap::new(),
        };
        let haps = poly.query(&state);
        eprintln!("Polyphonic pattern events:");
        for hap in &haps {
            eprintln!(
                "  {} : [{}, {}]",
                hap.value,
                hap.part.begin.to_float(),
                hap.part.end.to_float()
            );
        }

        // This should have overlapping events
        assert!(verify_polyphony(&poly));
    }
}
