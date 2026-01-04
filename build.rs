/// Build script to auto-generate tab completion metadata from source files
///
/// This script parses:
/// 1. src/nodes/*.rs - Audio node structs
/// 2. src/pattern.rs, src/pattern_ops.rs - Pattern methods
///
/// Extracts from standardized doc comments:
/// - Function name and description
/// - Parameters with types and defaults
/// - Example code
/// - Category
///
/// Generated output: src/modal_editor/completion/generated_metadata.rs
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/nodes/");
    println!("cargo:rerun-if-changed=src/pattern.rs");
    println!("cargo:rerun-if-changed=src/pattern_ops.rs");

    let mut all_metadata = Vec::new();

    // Parse node files (existing functionality)
    parse_node_files(&mut all_metadata);

    // Parse pattern method files (NEW)
    parse_pattern_files(&mut all_metadata);

    // Generate output file
    generate_metadata_file(&all_metadata);
}

/// Parse all src/nodes/*.rs files for node metadata
fn parse_node_files(all_metadata: &mut Vec<NodeMetadata>) {
    let nodes_dir = Path::new("src/nodes");
    let entries = match fs::read_dir(nodes_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Warning: Could not read src/nodes directory: {}", e);
            return;
        }
    };

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
}

/// Parse pattern method files with standardized doc comments
fn parse_pattern_files(all_metadata: &mut Vec<NodeMetadata>) {
    let files = ["src/pattern.rs", "src/pattern_ops.rs"];

    for file_path in &files {
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Could not read {}: {}", file_path, e);
                continue;
            }
        };

        let methods = parse_pattern_methods(&content);
        all_metadata.extend(methods);
    }
}

#[derive(Debug, Clone)]
struct NodeMetadata {
    name: String,        // e.g., "compressor", "fast"
    description: String, // Short description
    params: Vec<ParamInfo>,
    category: String,    // e.g., "Filters", "Transforms"
    example: String,     // Example code
}

#[derive(Debug, Clone)]
struct ParamInfo {
    name: String,
    param_type: String,   // "Hz", "float", "cycles", etc.
    default: Option<String>, // Default value if optional
    description: String,  // Parameter description
}

fn parse_node_file(content: &str, _file_name: &str) -> Option<NodeMetadata> {
    // Look for the struct definition
    // Pattern: "pub struct XxxNode {"
    let struct_pattern = "pub struct ".to_string();
    let struct_line = content
        .lines()
        .find(|line| line.trim().starts_with(&struct_pattern) && line.contains("Node"))?;

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

    let name = struct_name.strip_suffix("Node")?.to_lowercase();

    // Extract description from first line of struct doc comment
    let description = extract_node_description(content, struct_name)?;

    // Extract parameters from `pub fn new(...)` signature
    let params = extract_node_parameters(content)?;

    Some(NodeMetadata {
        name,
        description,
        params,
        category: "Effects".to_string(), // Default category for nodes
        example: String::new(),
    })
}

fn extract_node_description(content: &str, struct_name: &str) -> Option<String> {
    // Find the struct definition line
    let struct_line_index = content
        .lines()
        .position(|line| line.contains("pub struct") && line.contains(struct_name))?;

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
            continue;
        } else {
            break;
        }
    }

    desc_lines.reverse();
    desc_lines.first().map(|s| s.to_string())
}

fn extract_node_parameters(content: &str) -> Option<Vec<ParamInfo>> {
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
    let mut params = Vec::new();
    let param_section = signature.strip_prefix("pub fn new(")?.strip_suffix(")")?;

    for param in param_section.split(',') {
        let param = param.trim();
        if param.is_empty() || param == "self" || param == "&self" || param == "&mut self" {
            continue;
        }

        let parts: Vec<&str> = param.split(':').collect();
        if parts.len() < 2 {
            continue;
        }

        let param_name = parts[0].trim().to_string();
        let param_type = parts[1].trim().to_string();
        let clean_type = param_type
            .split('<')
            .next()
            .unwrap_or(&param_type)
            .trim()
            .to_string();

        params.push(ParamInfo {
            name: param_name,
            param_type: clean_type,
            default: None,
            description: String::new(),
        });
    }

    Some(params)
}

/// Parse pattern method doc comments following the standardized format
///
/// Looks for functions with doc comments containing:
/// - First line: short description
/// - # Parameters section
/// - # Example section
/// - # Category section
fn parse_pattern_methods(content: &str) -> Vec<NodeMetadata> {
    let mut methods = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        // Look for function definitions: "pub fn name(self"
        let line = lines[i].trim();
        if line.starts_with("pub fn ") && line.contains("(self") {
            // Found a method - look for doc comments above it
            if let Some(metadata) = parse_method_with_docs(&lines, i) {
                methods.push(metadata);
            }
        }
        i += 1;
    }

    methods
}

/// Parse a single method and its doc comments
fn parse_method_with_docs(lines: &[&str], fn_line_idx: usize) -> Option<NodeMetadata> {
    let fn_line = lines[fn_line_idx].trim();

    // Extract function name from "pub fn name(self..."
    let name = fn_line
        .strip_prefix("pub fn ")?
        .split('(')
        .next()?
        .trim()
        .to_string();

    // Skip internal functions (starting with _)
    if name.starts_with('_') {
        return None;
    }

    // Collect doc comment lines above the function
    let mut doc_lines = Vec::new();
    for i in (0..fn_line_idx).rev() {
        let line = lines[i].trim();
        if line.starts_with("///") {
            let text = line.strip_prefix("///").unwrap_or("").trim();
            doc_lines.push(text.to_string());
        } else if line.is_empty() {
            continue;
        } else {
            break;
        }
    }
    doc_lines.reverse();

    // Need at least a description
    if doc_lines.is_empty() {
        return None;
    }

    // Parse doc comment sections
    let description = doc_lines.first()?.clone();

    // Check if this has the standardized format (has # Category)
    let has_category = doc_lines.iter().any(|l| l.starts_with("# Category"));
    if !has_category {
        return None; // Skip methods without standardized docs
    }

    let params = parse_params_section(&doc_lines);
    let example = parse_example_section(&doc_lines);
    let category = parse_category_section(&doc_lines);

    Some(NodeMetadata {
        name,
        description,
        params,
        category,
        example,
    })
}

/// Parse # Parameters section from doc lines
fn parse_params_section(doc_lines: &[String]) -> Vec<ParamInfo> {
    let mut params = Vec::new();
    let mut in_params = false;

    for line in doc_lines {
        if line.starts_with("# Parameters") {
            in_params = true;
            continue;
        }
        if line.starts_with("# ") && in_params {
            break; // Hit next section
        }
        if in_params && line.starts_with("* `") {
            // Parse: * `name` - Description (type, default: value)
            if let Some(param) = parse_param_line(line) {
                params.push(param);
            }
        }
    }

    params
}

/// Parse a single parameter line: * `name` - Description (type, default: value)
fn parse_param_line(line: &str) -> Option<ParamInfo> {
    // Format: * `name` - Description (type, default: value)
    let after_star = line.strip_prefix("* `")?;
    let name_end = after_star.find('`')?;
    let name = after_star[..name_end].to_string();

    let rest = &after_star[name_end + 1..];
    let rest = rest.strip_prefix(" - ").unwrap_or(rest);

    // Find the parenthesized type info at the end
    let mut description = rest.to_string();
    let mut param_type = "unknown".to_string();
    let mut default = None;

    if let Some(paren_start) = rest.rfind('(') {
        if let Some(paren_end) = rest.rfind(')') {
            let type_info = &rest[paren_start + 1..paren_end];
            description = rest[..paren_start].trim().to_string();

            // Parse type info: "type, default: value" or "type, required"
            let parts: Vec<&str> = type_info.split(',').map(|s| s.trim()).collect();
            if !parts.is_empty() {
                param_type = parts[0].to_string();
            }
            if parts.len() > 1 {
                let default_part = parts[1];
                if default_part.starts_with("default:") {
                    default = Some(default_part.strip_prefix("default:")?.trim().to_string());
                }
            }
        }
    }

    Some(ParamInfo {
        name,
        param_type,
        default,
        description,
    })
}

/// Parse # Example section from doc lines
fn parse_example_section(doc_lines: &[String]) -> String {
    let mut in_example = false;
    let mut in_code_block = false;
    let mut example_lines = Vec::new();

    for line in doc_lines {
        if line.starts_with("# Example") {
            in_example = true;
            continue;
        }
        if line.starts_with("# ") && in_example && !line.starts_with("# Example") {
            break;
        }
        if in_example {
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            if in_code_block {
                example_lines.push(line.as_str());
            }
        }
    }

    example_lines.join("\n")
}

/// Parse # Category section from doc lines
fn parse_category_section(doc_lines: &[String]) -> String {
    let mut found_category = false;
    for line in doc_lines {
        if line.starts_with("# Category") {
            found_category = true;
            continue;
        }
        if found_category && !line.is_empty() {
            return line.clone();
        }
    }
    "Unknown".to_string()
}

fn generate_metadata_file(metadata: &[NodeMetadata]) {
    let mut output = String::from(
        r#"// AUTO-GENERATED FILE - DO NOT EDIT
// Generated by build.rs from source code doc comments
//
// Sources:
//   - src/nodes/*.rs (audio nodes)
//   - src/pattern.rs, src/pattern_ops.rs (pattern methods)
//
// To regenerate: cargo build (will trigger build.rs)

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GeneratedParamMetadata {
    pub name: String,
    pub param_type: String,
    pub default: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct GeneratedNodeMetadata {
    pub name: String,
    pub description: String,
    pub params: Vec<GeneratedParamMetadata>,
    pub category: String,
    pub example: String,
}

/// Get all auto-generated function metadata
pub fn get_all_functions() -> HashMap<String, GeneratedNodeMetadata> {
    let mut map = HashMap::new();

"#,
    );

    // Generate entries
    for node in metadata {
        output.push_str(&format!("    // {} ({})\n", node.name, node.category));
        output.push_str(&format!(
            "    map.insert(\"{}\".to_string(), GeneratedNodeMetadata {{\n",
            node.name
        ));
        output.push_str(&format!("        name: \"{}\".to_string(),\n", node.name));
        output.push_str(&format!(
            "        description: \"{}\".to_string(),\n",
            escape_string(&node.description)
        ));
        output.push_str(&format!(
            "        category: \"{}\".to_string(),\n",
            escape_string(&node.category)
        ));
        output.push_str(&format!(
            "        example: \"{}\".to_string(),\n",
            escape_string(&node.example)
        ));
        output.push_str("        params: vec![\n");

        for param in &node.params {
            output.push_str("            GeneratedParamMetadata {\n");
            output.push_str(&format!(
                "                name: \"{}\".to_string(),\n",
                param.name
            ));
            output.push_str(&format!(
                "                param_type: \"{}\".to_string(),\n",
                param.param_type
            ));
            if let Some(default) = &param.default {
                output.push_str(&format!(
                    "                default: Some(\"{}\".to_string()),\n",
                    escape_string(default)
                ));
            } else {
                output.push_str("                default: None,\n");
            }
            output.push_str(&format!(
                "                description: \"{}\".to_string(),\n",
                escape_string(&param.description)
            ));
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
        println!(
            "Generated completion metadata for {} functions",
            metadata.len()
        );
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
