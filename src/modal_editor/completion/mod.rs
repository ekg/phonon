//! Tab completion system for Phonon live coding editor
//!
//! Provides context-aware completion for:
//! - Function names (s, fast, lpf, etc.)
//! - Sample names (from ~/dirt-samples/)
//! - Bus references (~name)

mod context;
mod discovery;
mod function_metadata;
mod generated_metadata;
mod matching;
mod parameter;
mod state;

pub use context::{CompletionContext, get_completion_context, get_token_at_cursor};
pub use discovery::{discover_samples, extract_bus_names};
pub use function_metadata::{FunctionMetadata, FUNCTION_METADATA, search_functions, functions_by_category};
pub use matching::filter_completions;
pub use state::CompletionState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Basic smoke test to verify module structure
        let samples = vec!["bd".to_string(), "sn".to_string()];
        let buses = vec!["bass".to_string()];

        // Test context detection
        let context = get_completion_context("s \"bd", 5);
        assert!(matches!(context, CompletionContext::Sample));

        // Test completion filtering
        let completions = filter_completions("b", &context, &samples, &buses);
        assert!(!completions.is_empty());
    }
}
