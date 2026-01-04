//! Completion state machine
//!
//! Manages the state of the completion popup: visible/hidden, selected item, etc.

use super::matching::Completion;

/// Action to perform based on state transition
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionAction {
    /// Show the completion popup
    Show,
    /// Hide the completion popup
    Hide,
    /// Update the selected index
    UpdateSelection,
    /// Accept the current completion
    Accept,
    /// Cycle to next completion
    Next,
    /// Cycle to previous completion
    Previous,
    /// Visual feedback for no matches
    Flash,
}

/// State of the completion system
#[derive(Debug, Clone)]
pub struct CompletionState {
    /// Whether the popup is visible
    visible: bool,
    /// Whether the documentation panel is visible
    docs_panel_visible: bool,
    /// Available completions
    completions: Vec<Completion>,
    /// Currently selected index
    selected_index: usize,
    /// The original token being completed
    original_token: String,
    /// Start position of the token in the line
    token_start: usize,
}

impl CompletionState {
    /// Create a new empty completion state
    pub fn new() -> Self {
        Self {
            visible: false,
            docs_panel_visible: true, // Docs panel visible by default
            completions: Vec::new(),
            selected_index: 0,
            original_token: String::new(),
            token_start: 0,
        }
    }

    /// Check if the popup is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Check if the documentation panel is visible
    pub fn is_docs_panel_visible(&self) -> bool {
        self.docs_panel_visible
    }

    /// Toggle the documentation panel visibility
    pub fn toggle_docs_panel(&mut self) {
        self.docs_panel_visible = !self.docs_panel_visible;
    }

    /// Show the documentation panel
    pub fn show_docs_panel(&mut self) {
        self.docs_panel_visible = true;
    }

    /// Hide the documentation panel
    pub fn hide_docs_panel(&mut self) {
        self.docs_panel_visible = false;
    }

    /// Get the list of completions
    pub fn completions(&self) -> &[Completion] {
        &self.completions
    }

    /// Get the currently selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get the currently selected completion
    pub fn selected_completion(&self) -> Option<&Completion> {
        self.completions.get(self.selected_index)
    }

    /// Get the original token being completed
    pub fn original_token(&self) -> &str {
        &self.original_token
    }

    /// Get the token start position
    pub fn token_start(&self) -> usize {
        self.token_start
    }

    /// Show completions
    pub fn show(&mut self, completions: Vec<Completion>, token: String, token_start: usize) {
        self.visible = true;
        self.completions = completions;
        self.selected_index = 0;
        self.original_token = token;
        self.token_start = token_start;
    }

    /// Hide the popup
    pub fn hide(&mut self) {
        self.visible = false;
        self.completions.clear();
        self.selected_index = 0;
    }

    /// Move selection to next item (wrapping)
    pub fn next(&mut self) {
        if !self.completions.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.completions.len();
        }
    }

    /// Move selection to previous item (wrapping)
    pub fn previous(&mut self) {
        if !self.completions.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.completions.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Accept the current selection and hide
    pub fn accept(&mut self) -> Option<Completion> {
        let completion = self.selected_completion().cloned();
        self.hide();
        completion
    }
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modal_editor::completion::matching::CompletionType;

    fn make_completions() -> Vec<Completion> {
        vec![
            Completion::new("fast".to_string(), CompletionType::Function, None),
            Completion::new("fade".to_string(), CompletionType::Function, None),
            Completion::new("fadeIn".to_string(), CompletionType::Function, None),
        ]
    }

    #[test]
    fn test_initial_state() {
        let state = CompletionState::new();
        assert!(!state.is_visible());
        assert_eq!(state.completions().len(), 0);
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_show_completions() {
        let mut state = CompletionState::new();
        let completions = make_completions();

        state.show(completions.clone(), "fa".to_string(), 10);

        assert!(state.is_visible());
        assert_eq!(state.completions().len(), 3);
        assert_eq!(state.selected_index(), 0);
        assert_eq!(state.original_token(), "fa");
        assert_eq!(state.token_start(), 10);
    }

    #[test]
    fn test_hide_completions() {
        let mut state = CompletionState::new();
        state.show(make_completions(), "fa".to_string(), 10);

        state.hide();

        assert!(!state.is_visible());
        assert_eq!(state.completions().len(), 0);
    }

    #[test]
    fn test_next_navigation() {
        let mut state = CompletionState::new();
        state.show(make_completions(), "fa".to_string(), 10);

        assert_eq!(state.selected_index(), 0);

        state.next();
        assert_eq!(state.selected_index(), 1);

        state.next();
        assert_eq!(state.selected_index(), 2);

        // Should wrap
        state.next();
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_previous_navigation() {
        let mut state = CompletionState::new();
        state.show(make_completions(), "fa".to_string(), 10);

        assert_eq!(state.selected_index(), 0);

        // Should wrap to end
        state.previous();
        assert_eq!(state.selected_index(), 2);

        state.previous();
        assert_eq!(state.selected_index(), 1);

        state.previous();
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_selected_completion() {
        let mut state = CompletionState::new();
        state.show(make_completions(), "fa".to_string(), 10);

        let selected = state.selected_completion().unwrap();
        assert_eq!(selected.text, "fast");

        state.next();
        let selected = state.selected_completion().unwrap();
        assert_eq!(selected.text, "fade");
    }

    #[test]
    fn test_accept() {
        let mut state = CompletionState::new();
        state.show(make_completions(), "fa".to_string(), 10);

        state.next(); // Select "fade"

        let accepted = state.accept().unwrap();
        assert_eq!(accepted.text, "fade");

        // Should be hidden after accept
        assert!(!state.is_visible());
    }

    #[test]
    fn test_accept_empty() {
        let mut state = CompletionState::new();
        let accepted = state.accept();
        assert!(accepted.is_none());
    }

    #[test]
    fn test_docs_panel_visible_by_default() {
        let state = CompletionState::new();
        assert!(state.is_docs_panel_visible());
    }

    #[test]
    fn test_toggle_docs_panel() {
        let mut state = CompletionState::new();
        assert!(state.is_docs_panel_visible());

        state.toggle_docs_panel();
        assert!(!state.is_docs_panel_visible());

        state.toggle_docs_panel();
        assert!(state.is_docs_panel_visible());
    }

    #[test]
    fn test_show_hide_docs_panel() {
        let mut state = CompletionState::new();

        state.hide_docs_panel();
        assert!(!state.is_docs_panel_visible());

        state.show_docs_panel();
        assert!(state.is_docs_panel_visible());
    }
}
