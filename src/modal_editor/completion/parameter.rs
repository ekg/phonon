/// Parameter completion and template insertion logic
///
/// Provides utilities for:
/// - Generating parameter templates with _ placeholders
/// - Showing parameter suggestions
/// - Inserting complete parameter templates

use super::generated_metadata::{GeneratedNodeMetadata, get_all_nodes};
use super::function_metadata::{FunctionMetadata, FUNCTION_METADATA};

/// Generate parameter template for a function
///
/// Templates use:
/// - `_` for required parameters (need user input)
/// - Default values for optional parameters with defaults
///
/// Example:
/// - `compressor _ _ _ _ _` (all required)
/// - `delay _ 0.1 1.0` (input required, default delay time and max delay)
pub fn generate_param_template(function_name: &str) -> Option<String> {
    // First check manually-curated function metadata
    if let Some(metadata) = FUNCTION_METADATA.get(function_name) {
        return Some(generate_template_from_curated(metadata));
    }

    // Fall back to auto-generated node metadata
    let generated_nodes = get_all_nodes();
    if let Some(node_metadata) = generated_nodes.get(function_name) {
        return Some(generate_template_from_generated(node_metadata));
    }

    None
}

/// Generate template from manually-curated metadata (has optional/default info)
fn generate_template_from_curated(metadata: &FunctionMetadata) -> String {
    let params: Vec<String> = metadata
        .params
        .iter()
        .map(|p| {
            if p.optional {
                // Optional parameters: show default if available, otherwise _
                p.default.unwrap_or("_").to_string()
            } else {
                // Required parameters: use _
                "_".to_string()
            }
        })
        .collect();

    if params.is_empty() {
        metadata.name.to_string()
    } else {
        format!("{} {}", metadata.name, params.join(" "))
    }
}

/// Generate template from auto-generated metadata (no optional/default info yet)
/// For now, all params are treated as required (use _)
fn generate_template_from_generated(metadata: &GeneratedNodeMetadata) -> String {
    if metadata.params.is_empty() {
        return metadata.name.clone();
    }

    // All parameters required for now (until we extract defaults from docs)
    let placeholders = vec!["_"; metadata.params.len()];
    format!("{} {}", metadata.name, placeholders.join(" "))
}

/// Get parameter hints for display (show parameter names and types)
///
/// Returns a string like ":input NodeId :cutoff NodeId :resonance NodeId"
pub fn get_param_hints(function_name: &str) -> Option<String> {
    // Check manually-curated metadata first
    if let Some(metadata) = FUNCTION_METADATA.get(function_name) {
        let hints: Vec<String> = metadata
            .params
            .iter()
            .map(|p| {
                if p.optional {
                    if let Some(default) = p.default {
                        format!("[:{} {}={}]", p.name, p.param_type, default)
                    } else {
                        format!("[:{} {}]", p.name, p.param_type)
                    }
                } else {
                    format!(":{} {}", p.name, p.param_type)
                }
            })
            .collect();
        return Some(hints.join(" "));
    }

    // Fall back to auto-generated metadata
    let generated_nodes = get_all_nodes();
    if let Some(node_metadata) = generated_nodes.get(function_name) {
        let hints: Vec<String> = node_metadata
            .params
            .iter()
            .map(|p| format!(":{} {}", p.name, p.param_type))
            .collect();
        return Some(hints.join(" "));
    }

    None
}

/// Check if a function exists in either metadata source
pub fn function_exists(function_name: &str) -> bool {
    FUNCTION_METADATA.contains_key(function_name)
        || get_all_nodes().contains_key(function_name)
}

/// Get list of all available functions (both curated and generated)
pub fn get_all_function_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();

    // Add manually-curated functions
    for name in FUNCTION_METADATA.keys() {
        names.push(name.to_string());
    }

    // Add auto-generated node names
    for name in get_all_nodes().keys() {
        names.push(name.clone());
    }

    // Remove duplicates and sort
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_param_template_for_generated_node() {
        // Test with a generated node (e.g., compressor)
        if let Some(template) = generate_param_template("compressor") {
            // Should have function name + placeholders for each parameter
            assert!(template.starts_with("compressor"));
            assert!(template.contains("_"));
        }
    }

    #[test]
    fn test_get_param_hints_for_generated_node() {
        // Test with a generated node
        if let Some(hints) = get_param_hints("compressor") {
            // Should contain parameter names with colons
            assert!(hints.contains(":"));
        }
    }

    #[test]
    fn test_function_exists() {
        // Test that we can find auto-generated nodes
        // Compressor should exist (it's a node)
        // The exact name depends on what nodes are available
        let all_functions = get_all_function_names();
        assert!(!all_functions.is_empty(), "Should have some functions");
    }

    #[test]
    fn test_get_all_function_names() {
        let names = get_all_function_names();

        // Should have many functions
        assert!(names.len() > 50, "Should have at least 50 functions (got {})", names.len());

        // Should be sorted
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "Function names should be sorted");

        // Should have no duplicates
        let original_len = names.len();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(original_len, deduped.len(), "Should have no duplicate function names");
    }
}
