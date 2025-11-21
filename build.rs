/// Build script to auto-generate tab completion metadata from node source files
///
/// This script parses all src/nodes/*.rs files and extracts:
/// - Function names from struct names (e.g., CompressorNode → "compressor")
/// - Short descriptions from first line of doc comments
/// - Parameter names and types from `new()` function signatures
/// - Parameter descriptions from doc comments
///
/// Generated output: src/modal_editor/completion/generated_metadata.rs

use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/nodes/");

    // Get all node files
    let nodes_dir = Path::new("src/nodes");
    let entries = match fs::read_dir(nodes_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Warning: Could not read src/nodes directory: {}", e);
            return;
        }
    };

    let mut all_metadata = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
            // Skip mod.rs
            if file_name == "mod" {
                continue;
            }

            // Read file
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Parse the file
            if let Some(metadata) = parse_node_file(&content, file_name) {
                all_metadata.push(metadata);
            }
        }
    }

    // Generate output file
    generate_metadata_file(&all_metadata);
}

#[derive(Debug)]
struct NodeMetadata {
    name: String,           // e.g., "compressor"
    description: String,     // Short description
    params: Vec<ParamInfo>,
}

#[derive(Debug)]
struct ParamInfo {
    name: String,
    param_type: String,      // "NodeId", "Signal", "Pattern", etc.
    description: String,
}

fn parse_node_file(content: &str, file_name: &str) -> Option<NodeMetadata> {
    // Look for the struct definition
    // Pattern: "pub struct XxxNode {"
    let struct_pattern = format!("pub struct ");
    let struct_line = content.lines().find(|line| {
        line.trim().starts_with(&struct_pattern) && line.contains("Node")
    })?;

    // Extract struct name (e.g., "CompressorNode" → "compressor")
    let struct_name = struct_line
        .split("struct")
        .nth(1)?
        .trim()
        .split_whitespace()
        .next()?
        .trim();

    if !struct_name.ends_with("Node") {
        return None;
    }

    let name = struct_name
        .strip_suffix("Node")?
        .to_lowercase();

    // Extract description from first line of struct doc comment
    let description = extract_description(content, struct_name)?;

    // Extract parameters from `pub fn new(...)` signature
    let params = extract_parameters(content)?;

    Some(NodeMetadata {
        name,
        description,
        params,
    })
}

fn extract_description(content: &str, struct_name: &str) -> Option<String> {
    // Find the struct definition line
    let struct_line_index = content.lines().position(|line| {
        line.contains("pub struct") && line.contains(struct_name)
    })?;

    // Look backwards for doc comments
    let lines: Vec<&str> = content.lines().collect();
    let mut desc_lines = Vec::new();

    for i in (0..struct_line_index).rev() {
        let line = lines[i].trim();
        if line.starts_with("///") {
            let text = line.strip_prefix("///")?.trim();
            if !text.is_empty() {
                desc_lines.push(text);
            }
        } else if line.starts_with("//") || line.is_empty() {
            // Continue looking through non-doc comments and empty lines
            continue;
        } else {
            // Hit non-comment, stop
            break;
        }
    }

    // Reverse since we collected backwards
    desc_lines.reverse();

    // Take first meaningful line as description
    desc_lines.first().map(|s| s.to_string())
}

fn extract_parameters(content: &str) -> Option<Vec<ParamInfo>> {
    // Find the `pub fn new(` line
    let new_fn_start = content.find("pub fn new(")?;
    let after_new = &content[new_fn_start..];

    // Find the closing parenthesis of the function signature
    let mut paren_count = 0;
    let mut sig_end = 0;
    for (i, ch) in after_new.char_indices() {
        if ch == '(' {
            paren_count += 1;
        } else if ch == ')' {
            paren_count -= 1;
            if paren_count == 0 {
                sig_end = i;
                break;
            }
        }
    }

    if sig_end == 0 {
        return None;
    }

    let signature = &after_new[..=sig_end];

    // Extract parameters from signature
    // Pattern: "param_name: Type"
    let mut params = Vec::new();

    // Split by commas, but be careful of nested generics
    let param_section = signature
        .strip_prefix("pub fn new(")?
        .strip_suffix(")")?;

    for param in param_section.split(',') {
        let param = param.trim();

        // Skip empty, self, and &self
        if param.is_empty() || param == "self" || param == "&self" || param == "&mut self" {
            continue;
        }

        // Parse "name: Type"
        let parts: Vec<&str> = param.split(':').collect();
        if parts.len() < 2 {
            continue;
        }

        let param_name = parts[0].trim().to_string();
        let param_type = parts[1].trim().to_string();

        // Extract just the type name (remove generic parameters)
        let clean_type = param_type
            .split('<')
            .next()
            .unwrap_or(&param_type)
            .trim()
            .to_string();

        params.push(ParamInfo {
            name: param_name,
            param_type: clean_type,
            description: String::new(), // TODO: Extract from doc comments if needed
        });
    }

    Some(params)
}

fn generate_metadata_file(metadata: &[NodeMetadata]) {
    let mut output = String::from(
r#"// AUTO-GENERATED FILE - DO NOT EDIT
// Generated by build.rs from src/nodes/*.rs doc comments
//
// To regenerate: cargo build (will trigger build.rs)

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GeneratedParamMetadata {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Clone)]
pub struct GeneratedNodeMetadata {
    pub name: String,
    pub description: String,
    pub params: Vec<GeneratedParamMetadata>,
}

/// Get all auto-generated node metadata
pub fn get_all_nodes() -> HashMap<String, GeneratedNodeMetadata> {
    let mut map = HashMap::new();

"#
    );

    // Generate entries
    for node in metadata {
        output.push_str(&format!("    // {}\n", node.name));
        output.push_str(&format!("    map.insert(\"{}\".to_string(), GeneratedNodeMetadata {{\n", node.name));
        output.push_str(&format!("        name: \"{}\".to_string(),\n", node.name));
        output.push_str(&format!("        description: \"{}\".to_string(),\n", escape_string(&node.description)));
        output.push_str("        params: vec![\n");

        for param in &node.params {
            output.push_str("            GeneratedParamMetadata {\n");
            output.push_str(&format!("                name: \"{}\".to_string(),\n", param.name));
            output.push_str(&format!("                param_type: \"{}\".to_string(),\n", param.param_type));
            output.push_str("            },\n");
        }

        output.push_str("        ],\n");
        output.push_str("    });\n\n");
    }

    output.push_str("    map\n");
    output.push_str("}\n");

    // Write to file
    let out_path = Path::new("src/modal_editor/completion/generated_metadata.rs");
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Err(e) = fs::write(out_path, output) {
        eprintln!("Warning: Could not write generated metadata: {}", e);
    } else {
        println!("Generated completion metadata for {} nodes", metadata.len());
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
