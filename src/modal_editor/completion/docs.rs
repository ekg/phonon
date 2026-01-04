//! Documentation retrieval for completion items
//!
//! Provides full documentation for functions including:
//! - Short description
//! - Long description (if available)
//! - Parameters with types and defaults
//! - Example code
//! - Category

use super::function_metadata::FUNCTION_METADATA;
use super::generated_metadata::get_all_functions;

/// Full documentation for a function
#[derive(Debug, Clone)]
pub struct FunctionDocs {
    /// Function name
    pub name: String,
    /// Short description (first line)
    pub short_description: String,
    /// Category (Filters, Effects, Transforms, etc.)
    pub category: String,
    /// Parameters with documentation
    pub params: Vec<ParamDoc>,
    /// Example code (if available)
    pub example: Option<String>,
}

/// Documentation for a single parameter
#[derive(Debug, Clone)]
pub struct ParamDoc {
    /// Parameter name
    pub name: String,
    /// Parameter type (Hz, float, 0-1, etc.)
    pub param_type: String,
    /// Default value (if optional)
    pub default: Option<String>,
    /// Description of the parameter
    pub description: String,
}

impl FunctionDocs {
    /// Get documentation for a function by name
    ///
    /// Combines information from curated metadata and generated metadata
    pub fn get(function_name: &str) -> Option<Self> {
        let curated = FUNCTION_METADATA.get(function_name);
        let generated = get_all_functions();
        let gen = generated.get(function_name);

        // Need at least one source
        if curated.is_none() && gen.is_none() {
            return None;
        }

        // Get description (prefer curated)
        let short_description = curated
            .map(|m| m.description.to_string())
            .or_else(|| gen.map(|g| g.description.clone()))
            .unwrap_or_default();

        // Get category (prefer curated)
        let category = curated
            .map(|m| m.category.to_string())
            .or_else(|| gen.map(|g| g.category.clone()))
            .unwrap_or_else(|| "Unknown".to_string());

        // Get example (only from generated for now)
        let example = gen
            .map(|g| g.example.clone())
            .filter(|e| !e.is_empty());

        // Get parameters (prefer curated, has more detail)
        let params = if let Some(m) = curated {
            m.params
                .iter()
                .map(|p| ParamDoc {
                    name: p.name.to_string(),
                    param_type: p.param_type.to_string(),
                    default: p.default.map(|d| d.to_string()),
                    description: p.description.to_string(),
                })
                .collect()
        } else if let Some(g) = gen {
            g.params
                .iter()
                .map(|p| ParamDoc {
                    name: p.name.clone(),
                    param_type: p.param_type.clone(),
                    default: p.default.clone(),
                    description: p.description.clone(),
                })
                .collect()
        } else {
            vec![]
        };

        Some(FunctionDocs {
            name: function_name.to_string(),
            short_description,
            category,
            params,
            example,
        })
    }

    /// Format documentation as lines for display
    ///
    /// Returns a vector of (text, is_header) pairs for styling
    pub fn format_lines(&self, max_width: usize) -> Vec<DocLine> {
        let mut lines = Vec::new();

        // Header: name - description [category]
        lines.push(DocLine::header(format!(
            "{} - {}",
            self.name, self.short_description
        )));
        lines.push(DocLine::empty());

        // Parameters section
        if !self.params.is_empty() {
            lines.push(DocLine::subheader("Parameters:".to_string()));

            for param in &self.params {
                let default_str = param
                    .default
                    .as_ref()
                    .map(|d| format!(" (default: {})", d))
                    .unwrap_or_default();

                // Format: "  name    type    description (default: value)"
                let param_line = format!(
                    "  {:12} {:8} {}{}",
                    param.name, param.param_type, param.description, default_str
                );

                // Truncate if too long
                let truncated = if param_line.len() > max_width {
                    format!("{}...", &param_line[..max_width.saturating_sub(3)])
                } else {
                    param_line
                };

                lines.push(DocLine::param(truncated));
            }

            lines.push(DocLine::empty());
        }

        // Example section
        if let Some(example) = &self.example {
            lines.push(DocLine::subheader("Example:".to_string()));
            lines.push(DocLine::example(format!("  {}", example)));
        }

        lines
    }
}

/// A line of documentation with styling information
#[derive(Debug, Clone)]
pub struct DocLine {
    pub text: String,
    pub style: DocLineStyle,
}

/// Style for a documentation line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocLineStyle {
    Header,
    Subheader,
    Param,
    Example,
    Empty,
}

impl DocLine {
    pub fn header(text: String) -> Self {
        Self {
            text,
            style: DocLineStyle::Header,
        }
    }

    pub fn subheader(text: String) -> Self {
        Self {
            text,
            style: DocLineStyle::Subheader,
        }
    }

    pub fn param(text: String) -> Self {
        Self {
            text,
            style: DocLineStyle::Param,
        }
    }

    pub fn example(text: String) -> Self {
        Self {
            text,
            style: DocLineStyle::Example,
        }
    }

    pub fn empty() -> Self {
        Self {
            text: String::new(),
            style: DocLineStyle::Empty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_docs_for_curated_function() {
        // lpf should have curated metadata
        let docs = FunctionDocs::get("lpf");
        assert!(docs.is_some());

        let docs = docs.unwrap();
        assert_eq!(docs.name, "lpf");
        assert!(docs.short_description.contains("Low-pass"));
        assert_eq!(docs.category, "Filters");
        assert!(!docs.params.is_empty());
    }

    #[test]
    fn test_get_docs_for_generated_function() {
        // compressor should have generated metadata
        let docs = FunctionDocs::get("compressor");
        assert!(docs.is_some());

        let docs = docs.unwrap();
        assert_eq!(docs.name, "compressor");
    }

    #[test]
    fn test_get_docs_nonexistent() {
        let docs = FunctionDocs::get("nonexistent_function_xyz");
        assert!(docs.is_none());
    }

    #[test]
    fn test_format_lines() {
        let docs = FunctionDocs::get("lpf").unwrap();
        let lines = docs.format_lines(80);

        // Should have header, params, etc.
        assert!(!lines.is_empty());

        // First line should be header
        assert_eq!(lines[0].style, DocLineStyle::Header);
        assert!(lines[0].text.contains("lpf"));
    }

    #[test]
    fn test_param_docs() {
        let docs = FunctionDocs::get("lpf").unwrap();

        // lpf should have cutoff and q parameters
        assert!(docs.params.iter().any(|p| p.name == "cutoff"));
        assert!(docs.params.iter().any(|p| p.name == "q"));
    }
}
