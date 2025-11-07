//! Sample and bus discovery
//!
//! Discovers available sample names and bus definitions

use std::fs;
use std::path::PathBuf;

/// Discover sample names from multiple possible locations
///
/// Searches in order:
/// 1. ~/dirt-samples/
/// 2. ~/phonon/dirt-samples/
/// 3. ./dirt-samples/ (relative to current directory)
///
/// Returns a sorted list of directory names (sample banks) found
pub fn discover_samples() -> Vec<String> {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => {
            eprintln!("Warning: HOME environment variable not set, no samples available");
            return Vec::new();
        }
    };

    // Try multiple locations
    let search_paths = vec![
        PathBuf::from(&home).join("dirt-samples"),
        PathBuf::from(&home).join("phonon").join("dirt-samples"),
        PathBuf::from(".").join("dirt-samples"),
    ];

    let dirt_samples = search_paths.iter()
        .find(|path| path.exists())
        .cloned();

    let dirt_samples = match dirt_samples {
        Some(path) => path,
        None => {
            eprintln!(
                "Warning: dirt-samples not found in any of: ~/dirt-samples, ~/phonon/dirt-samples, ./dirt-samples"
            );
            return Vec::new();
        }
    };

    let mut samples = Vec::new();

    match fs::read_dir(&dirt_samples) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            samples.push(name.to_string());
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to read ~/dirt-samples: {}", e);
            return Vec::new();
        }
    }

    samples.sort();
    samples
}

/// Extract bus names from editor content
///
/// Scans for lines matching the pattern: `~name:`
/// Returns a sorted, deduplicated list of bus names (without the ~ prefix)
pub fn extract_bus_names(content: &str) -> Vec<String> {
    let mut buses = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Look for ~name: pattern
        if trimmed.starts_with('~') {
            if let Some(colon_pos) = trimmed.find(':') {
                let name = &trimmed[1..colon_pos];

                // Validate it's a valid identifier
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    buses.push(name.to_string());
                }
            }
        }
    }

    buses.sort();
    buses.dedup();
    buses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_samples() {
        // This test depends on ~/dirt-samples existing
        // Just verify it returns a Vec without panicking
        let samples = discover_samples();

        // If ~/dirt-samples exists, we should have samples
        // Otherwise we get an empty vec
        if !samples.is_empty() {
            // Verify they're sorted
            let mut sorted = samples.clone();
            sorted.sort();
            assert_eq!(samples, sorted);

            // Verify no duplicates
            let unique_count = samples.iter().collect::<std::collections::HashSet<_>>().len();
            assert_eq!(samples.len(), unique_count);
        }
    }

    #[test]
    fn test_extract_bus_names_simple() {
        let content = "~bass: saw 55\n~drums: s \"bd sn\"";

        let buses = extract_bus_names(content);

        assert_eq!(buses.len(), 2);
        assert!(buses.contains(&"bass".to_string()));
        assert!(buses.contains(&"drums".to_string()));
    }

    #[test]
    fn test_extract_bus_names_with_whitespace() {
        let content = "  ~bass:   saw 55  \n\t~drums:\ts \"bd sn\"";

        let buses = extract_bus_names(content);

        assert_eq!(buses.len(), 2);
        assert!(buses.contains(&"bass".to_string()));
        assert!(buses.contains(&"drums".to_string()));
    }

    #[test]
    fn test_extract_bus_names_empty() {
        let content = "out: s \"bd sn\"";

        let buses = extract_bus_names(content);

        assert!(buses.is_empty());
    }

    #[test]
    fn test_extract_bus_names_duplicates() {
        let content = "~bass: saw 55\n~bass: saw 82.5\n~drums: s \"bd\"";

        let buses = extract_bus_names(content);

        // Should deduplicate
        assert_eq!(buses.len(), 2);
        assert!(buses.contains(&"bass".to_string()));
        assert!(buses.contains(&"drums".to_string()));
    }

    #[test]
    fn test_extract_bus_names_sorted() {
        let content = "~zebra: saw 55\n~alpha: saw 82.5\n~beta: s \"bd\"";

        let buses = extract_bus_names(content);

        assert_eq!(buses, vec!["alpha", "beta", "zebra"]);
    }

    #[test]
    fn test_extract_bus_names_invalid() {
        // Invalid: spaces in name, special chars
        let content = "~invalid name: saw 55\n~invalid-name: saw 82.5\n~valid: s \"bd\"";

        let buses = extract_bus_names(content);

        // Should only find valid
        assert_eq!(buses.len(), 1);
        assert!(buses.contains(&"valid".to_string()));
    }

    #[test]
    fn test_extract_bus_names_with_underscores() {
        let content = "~bass_line: saw 55\n~drum_loop: s \"bd\"";

        let buses = extract_bus_names(content);

        assert_eq!(buses.len(), 2);
        assert!(buses.contains(&"bass_line".to_string()));
        assert!(buses.contains(&"drum_loop".to_string()));
    }

    #[test]
    fn test_extract_bus_names_empty_content() {
        let content = "";

        let buses = extract_bus_names(content);

        assert!(buses.is_empty());
    }

    #[test]
    fn test_extract_bus_names_with_comments() {
        let content = "-- This is a comment\n~bass: saw 55\n-- Another comment\n~drums: s \"bd\"";

        let buses = extract_bus_names(content);

        assert_eq!(buses.len(), 2);
        assert!(buses.contains(&"bass".to_string()));
        assert!(buses.contains(&"drums".to_string()));
    }
}
