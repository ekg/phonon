//! Test harness for modal editor integration tests
//!
//! Provides a testable interface to the editor without requiring
//! terminal UI or audio output.

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Test harness for scripting editor interactions
pub struct EditorTestHarness {
    editor: ModalEditor,
}

impl EditorTestHarness {
    /// Create a new test harness with an empty editor
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut editor = ModalEditor::new(0.0, None)?;
        // Clear the default template content for clean testing
        editor.content = String::new();
        editor.cursor_pos = 0;
        Ok(Self { editor })
    }

    /// Create a test harness with initial content
    pub fn with_content(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut harness = Self::new()?;
        harness.editor.content = content.to_string();
        harness.editor.cursor_pos = content.len();
        Ok(harness)
    }

    /// Send a string of characters to the editor
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for ch in text.chars() {
            let event = KeyEvent::new(
                KeyCode::Char(ch),
                KeyModifiers::NONE,
            );
            self.editor.handle_key_event(event);
        }
        self
    }

    /// Send a key event
    pub fn send_key(&mut self, code: KeyCode) -> &mut Self {
        self.send_key_with_modifiers(code, KeyModifiers::NONE)
    }

    /// Send a key event with modifiers
    pub fn send_key_with_modifiers(&mut self, code: KeyCode, modifiers: KeyModifiers) -> &mut Self {
        let event = KeyEvent::new(code, modifiers);
        self.editor.handle_key_event(event);
        self
    }

    /// Send Tab key
    pub fn tab(&mut self) -> &mut Self {
        self.send_key(KeyCode::Tab)
    }

    /// Send Enter key
    pub fn enter(&mut self) -> &mut Self {
        self.send_key(KeyCode::Enter)
    }

    /// Send Backspace key
    pub fn backspace(&mut self) -> &mut Self {
        self.send_key(KeyCode::Backspace)
    }

    /// Send Ctrl+Space (for kwargs expansion)
    pub fn ctrl_space(&mut self) -> &mut Self {
        self.send_key_with_modifiers(KeyCode::Char(' '), KeyModifiers::CONTROL)
    }

    /// Get the current line content
    pub fn current_line(&self) -> &str {
        let lines: Vec<&str> = self.editor.content.lines().collect();
        let line_num = self.editor.content[..self.editor.cursor_pos]
            .chars()
            .filter(|&c| c == '\n')
            .count();
        lines.get(line_num).unwrap_or(&"")
    }

    /// Get all content
    pub fn content(&self) -> &str {
        &self.editor.content
    }

    /// Get cursor position
    pub fn cursor_pos(&self) -> usize {
        self.editor.cursor_pos
    }

    /// Check if completion dialog is shown
    pub fn is_completion_shown(&self) -> bool {
        self.editor.completion_state.is_visible()
    }

    /// Get completion options currently shown
    pub fn completion_options(&self) -> Vec<String> {
        self.editor.completion_state.completions()
            .iter()
            .map(|c| c.text.clone())
            .collect()
    }

    /// Get the selected completion (if any)
    pub fn selected_completion(&self) -> Option<String> {
        self.editor.completion_state.selected_completion()
            .map(|c| c.text.clone())
    }

    /// Assert that the current line equals expected
    pub fn assert_line(&mut self, expected: &str) -> &mut Self {
        let actual = self.current_line();
        assert_eq!(
            actual, expected,
            "\nExpected line: {:?}\nActual line: {:?}",
            expected, actual
        );
        self
    }

    /// Assert that completion is showing
    pub fn assert_completion_shown(&mut self) -> &mut Self {
        assert!(
            self.is_completion_shown(),
            "Expected completion dialog to be shown, but it's hidden"
        );
        self
    }

    /// Assert that completion is hidden
    pub fn assert_completion_hidden(&mut self) -> &mut Self {
        assert!(
            !self.is_completion_shown(),
            "Expected completion dialog to be hidden, but it's shown"
        );
        self
    }

    /// Assert that completion contains specific option
    pub fn assert_completion_contains(&mut self, option: &str) -> &mut Self {
        let options = self.completion_options();
        assert!(
            options.contains(&option.to_string()),
            "Expected completion to contain {:?}, but got: {:?}",
            option, options
        );
        self
    }

    /// Assert that completion options match exactly
    pub fn assert_completion_options(&mut self, expected: &[&str]) -> &mut Self {
        let actual: Vec<String> = self.completion_options();
        let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            actual, expected,
            "\nExpected completions: {:?}\nActual completions: {:?}",
            expected, actual
        );
        self
    }

    /// Assert that the selected completion matches
    pub fn assert_selected(&mut self, expected: &str) -> &mut Self {
        let selected = self.selected_completion();
        assert_eq!(
            selected, Some(expected.to_string()),
            "\nExpected selected: {:?}\nActual selected: {:?}",
            Some(expected), selected
        );
        self
    }

    /// Print current state for debugging
    pub fn debug_state(&mut self) -> &mut Self {
        eprintln!("=== Editor State ===");
        eprintln!("Content: {:?}", self.content());
        eprintln!("Current line: {:?}", self.current_line());
        eprintln!("Cursor pos: {}", self.cursor_pos());
        eprintln!("Completion shown: {}", self.is_completion_shown());
        if self.is_completion_shown() {
            eprintln!("Completion options: {:?}", self.completion_options());
            eprintln!("Selected: {:?}", self.selected_completion());
        }
        eprintln!("===================");
        self
    }

    /// Send Ctrl+X (evaluate chunk)
    pub fn ctrl_x(&mut self) -> &mut Self {
        self.send_key_with_modifiers(KeyCode::Char('x'), KeyModifiers::CONTROL)
    }

    /// Get CPS from the current graph (if loaded)
    pub fn get_cps(&self) -> Option<f32> {
        let graph_snapshot = self.editor.graph.load();
        if let Some(ref graph_cell) = **graph_snapshot {
            if let Ok(g) = graph_cell.0.try_borrow() {
                return Some(g.get_cps());
            }
        }
        None
    }

    /// Get cycle position from the current graph (if loaded)
    pub fn get_cycle_position(&self) -> Option<f64> {
        let graph_snapshot = self.editor.graph.load();
        if let Some(ref graph_cell) = **graph_snapshot {
            if let Ok(g) = graph_cell.0.try_borrow() {
                return Some(g.get_cycle_position());
            }
        }
        None
    }

    /// Check if wall-clock timing is enabled
    pub fn is_wall_clock_enabled(&self) -> Option<bool> {
        let graph_snapshot = self.editor.graph.load();
        if let Some(ref graph_cell) = **graph_snapshot {
            if let Ok(g) = graph_cell.0.try_borrow() {
                return Some(g.use_wall_clock);
            }
        }
        None
    }

    /// Check if a graph is loaded
    pub fn has_graph(&self) -> bool {
        let graph_snapshot = self.editor.graph.load();
        graph_snapshot.is_some()
    }

    /// Set content directly (for test setup)
    pub fn set_content(&mut self, content: &str) -> &mut Self {
        self.editor.content = content.to_string();
        self.editor.cursor_pos = content.len();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_basic_typing() {
        let mut harness = EditorTestHarness::new().unwrap();
        harness.type_text("hello")
            .assert_line("hello");
    }

    #[test]
    fn test_harness_multiline() {
        let mut harness = EditorTestHarness::new().unwrap();
        harness.type_text("line1")
            .enter()
            .type_text("line2")
            .assert_line("line2");
    }

    #[test]
    fn test_harness_with_initial_content() {
        let mut harness = EditorTestHarness::with_content("initial text").unwrap();
        harness.assert_line("initial text");
    }
}
