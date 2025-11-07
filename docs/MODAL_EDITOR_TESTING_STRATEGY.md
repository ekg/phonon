# Modal Editor Testing & Implementation Strategy

## Overview

This document outlines the comprehensive testing and implementation strategy for modal editor features, specifically syntax highlighting and tab completion. The goal is to achieve high test coverage (~90%) for editor logic while acknowledging that full TUI rendering cannot be tested in CI.

## Problem Statement

The modal editor is a Ratatui-based TUI application with:
- Event loop and keyboard handling
- Terminal rendering
- Complex stateful interactions (completion popups, navigation)

Testing challenges:
- Cannot easily test terminal rendering in CI
- Event handling is tightly coupled to UI
- State management spans multiple interactions

## Solution: Separate Pure Logic from UI

### Architecture

```
src/modal_editor/
  mod.rs              // Main ModalEditor struct (UI integration)
  highlighting.rs     // Pure syntax highlighting logic
  completion.rs       // Pure completion matching and filtering
  completion_state.rs // Completion popup state machine
  token.rs            // Token extraction and context detection
```

## Part 1: Syntax Highlighting Testing

### 1.1 Extract Pure Function

**Current** (in `modal_editor.rs`):
```rust
impl ModalEditor {
    fn highlight_line(line: &str) -> Vec<Span> {
        // ... 150 lines of highlighting logic
    }
}
```

**Refactored** (`modal_editor/highlighting.rs`):
```rust
/// Pure function: takes a line, returns styled spans
/// Easily testable without TUI infrastructure
pub fn highlight_line(line: &str) -> Vec<Span<'static>> {
    // Same logic, but extracted
}

// Keep function list as constant for reuse
pub const FUNCTIONS: &[&str] = &[
    "s", "stack", "fast", "slow", "gain", "pan",
    // ... all functions
];
```

### 1.2 Comprehensive Test Suite

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_highlight_functions() {
        let spans = highlight_line("fast 2");
        assert_eq!(spans.len(), 3); // "fast", " ", "2"

        // First span should be "fast" in blue
        assert_eq!(spans[0].content, "fast");
        assert_eq!(spans[0].style.fg, Some(Color::Blue));

        // Last span should be "2" in orange
        assert_eq!(spans[2].content, "2");
        assert_eq!(spans[2].style.fg, Some(Color::Rgb(255, 165, 0)));
    }

    #[test]
    fn test_highlight_chain_operator() {
        let spans = highlight_line("s \"bd\" # gain 0.5");

        // Find the '#' span
        let hash_span = spans.iter()
            .find(|s| s.content == "#")
            .expect("Should have # operator");

        assert_eq!(hash_span.style.fg, Some(Color::Rgb(255, 20, 147))); // Hot pink
    }

    #[test]
    fn test_highlight_dollar_operator() {
        let spans = highlight_line("s \"bd\" $ fast 2");

        let dollar_span = spans.iter()
            .find(|s| s.content == "$")
            .expect("Should have $ operator");

        assert_eq!(dollar_span.style.fg, Some(Color::Rgb(255, 20, 147))); // Hot pink
    }

    #[test]
    fn test_highlight_bus_reference() {
        let spans = highlight_line("~drums: s \"bd\"");

        let bus_span = spans.iter()
            .find(|s| s.content.starts_with('~'))
            .expect("Should have bus reference");

        assert_eq!(bus_span.style.fg, Some(Color::Magenta));
    }

    #[test]
    fn test_highlight_comment_line() {
        let spans = highlight_line("-- This is a comment");

        // Entire line should be one gray span
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg, Some(Color::Rgb(100, 100, 100)));
    }

    #[test]
    fn test_highlight_string_literal() {
        let spans = highlight_line("s \"bd sn hh\"");

        let string_span = spans.iter()
            .find(|s| s.content.contains("bd"))
            .expect("Should have string content");

        assert_eq!(string_span.style.fg, Some(Color::White));
    }

    #[test]
    fn test_highlight_number() {
        let spans = highlight_line("gain 0.5");

        let num_span = spans.iter()
            .find(|s| s.content == "0.5")
            .expect("Should have number");

        assert_eq!(num_span.style.fg, Some(Color::Rgb(255, 165, 0)));
    }

    #[test]
    fn test_highlight_all_new_functions() {
        // Verify all newly added functions are highlighted
        let new_functions = ["gain", "stack", "dist", "comp", "adsr"];

        for func in new_functions {
            let line = format!("{} 0.5", func);
            let spans = highlight_line(&line);

            let func_span = spans.iter()
                .find(|s| s.content == func)
                .expect(&format!("Should highlight {}", func));

            assert_eq!(func_span.style.fg, Some(Color::Blue),
                "{} should be highlighted as function", func);
        }
    }

    #[test]
    fn test_highlight_preserves_hash_not_comment() {
        // Critical: # is NOT a comment in Phonon, it's the chain operator
        let spans = highlight_line("# gain 0.5");

        // Should NOT be treated as comment (would be single gray span)
        assert!(spans.len() > 1, "# should not trigger comment mode");

        let hash_span = spans.iter()
            .find(|s| s.content == "#")
            .expect("Should have # operator");

        assert_eq!(hash_span.style.fg, Some(Color::Rgb(255, 20, 147)));
    }
}
```

### 1.3 Integration into ModalEditor

```rust
// modal_editor/mod.rs
use highlighting::highlight_line;

impl ModalEditor {
    // Just call the pure function
    fn render_content(&self) -> Vec<Line> {
        self.content
            .lines()
            .map(|line| Line::from(highlight_line(line)))
            .collect()
    }
}
```

## Part 2: Tab Completion Testing

### 2.1 Pure Logic Functions

**File: `modal_editor/token.rs`**
```rust
#[derive(Debug, PartialEq)]
pub struct Token {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, PartialEq)]
pub enum CompletionContext {
    Function,        // Outside strings: function names
    Sample,          // Inside s "...": sample names
    Bus,             // Inside strings starting with ~: bus references
    None,
}

/// Extract the token at cursor position
pub fn get_token_at_cursor(line: &str, cursor: usize) -> Option<Token> {
    if cursor > line.len() {
        return None;
    }

    // Find token boundaries
    let start = line[..cursor]
        .rfind(|c: char| c.is_whitespace() || "()[]{}\"#$".contains(c))
        .map(|i| i + 1)
        .unwrap_or(0);

    let end = line[cursor..]
        .find(|c: char| c.is_whitespace() || "()[]{}\"#$".contains(c))
        .map(|i| cursor + i)
        .unwrap_or(line.len());

    Some(Token {
        text: line[start..end].to_string(),
        start,
        end,
    })
}

/// Determine completion context based on cursor position
pub fn get_completion_context(line: &str, cursor: usize) -> CompletionContext {
    // Check if we're inside a string
    let before_cursor = &line[..cursor];
    let quote_count = before_cursor.matches('"').count();
    let inside_string = quote_count % 2 == 1;

    if !inside_string {
        return CompletionContext::Function;
    }

    // Inside a string - check if it's an s "..." pattern
    // Look backwards for 's "'
    if let Some(s_pos) = before_cursor.rfind("s \"") {
        // We're in a sample pattern string
        let token = get_token_at_cursor(line, cursor);
        if let Some(t) = token {
            if t.text.starts_with('~') {
                return CompletionContext::Bus;
            }
        }
        return CompletionContext::Sample;
    }

    CompletionContext::None
}
```

**Tests for token.rs:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_token_at_cursor_middle() {
        let token = get_token_at_cursor("fast 2", 2).unwrap();
        assert_eq!(token.text, "fast");
        assert_eq!(token.start, 0);
        assert_eq!(token.end, 4);
    }

    #[test]
    fn test_get_token_at_cursor_start() {
        let token = get_token_at_cursor("fast 2", 0).unwrap();
        assert_eq!(token.text, "fast");
    }

    #[test]
    fn test_get_token_partial() {
        let token = get_token_at_cursor("s \"bd\" # ga", 11).unwrap();
        assert_eq!(token.text, "ga");
    }

    #[test]
    fn test_completion_context_function() {
        let ctx = get_completion_context("fas", 3);
        assert_eq!(ctx, CompletionContext::Function);
    }

    #[test]
    fn test_completion_context_sample() {
        let ctx = get_completion_context("s \"bd", 5);
        assert_eq!(ctx, CompletionContext::Sample);
    }

    #[test]
    fn test_completion_context_bus() {
        let ctx = get_completion_context("s \"~drum", 8);
        assert_eq!(ctx, CompletionContext::Bus);
    }

    #[test]
    fn test_completion_context_outside_string() {
        let ctx = get_completion_context("s \"bd\" # ga", 11);
        assert_eq!(ctx, CompletionContext::Function);
    }
}
```

**File: `modal_editor/completion.rs`**
```rust
use super::token::*;
use super::highlighting::FUNCTIONS;

/// Filter candidates based on partial match
pub fn filter_completions(partial: &str, candidates: &[&str]) -> Vec<String> {
    candidates
        .iter()
        .filter(|c| c.starts_with(partial))
        .map(|s| s.to_string())
        .collect()
}

/// Get completion matches for current context
pub fn get_completions(
    line: &str,
    cursor: usize,
    sample_names: &[String],
    bus_names: &[String],
) -> Vec<String> {
    let context = get_completion_context(line, cursor);
    let token = get_token_at_cursor(line, cursor);

    let partial = token.map(|t| t.text).unwrap_or_default();

    match context {
        CompletionContext::Function => {
            filter_completions(&partial, FUNCTIONS)
        }
        CompletionContext::Sample => {
            let sample_refs: Vec<&str> = sample_names.iter()
                .map(|s| s.as_str())
                .collect();
            filter_completions(&partial, &sample_refs)
        }
        CompletionContext::Bus => {
            // Remove ~ from partial if present for matching
            let partial_without_tilde = partial.trim_start_matches('~');
            let bus_refs: Vec<&str> = bus_names.iter()
                .map(|s| s.as_str())
                .collect();
            filter_completions(partial_without_tilde, &bus_refs)
                .into_iter()
                .map(|s| format!("~{}", s))
                .collect()
        }
        CompletionContext::None => Vec::new(),
    }
}

/// Apply completion to line and return new line + cursor position
pub fn apply_completion(
    line: &str,
    cursor: usize,
    completion: &str,
) -> (String, usize) {
    let token = get_token_at_cursor(line, cursor);

    if let Some(t) = token {
        let mut new_line = String::new();
        new_line.push_str(&line[..t.start]);
        new_line.push_str(completion);
        new_line.push_str(&line[t.end..]);

        let new_cursor = t.start + completion.len();
        (new_line, new_cursor)
    } else {
        (line.to_string(), cursor)
    }
}
```

**Tests for completion.rs:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_completions() {
        let candidates = vec!["fast", "filter", "fade", "gain"];
        let matches = filter_completions("fa", &candidates);
        assert_eq!(matches, vec!["fast", "fade"]);
    }

    #[test]
    fn test_get_completions_function() {
        let samples = vec![];
        let buses = vec![];

        let matches = get_completions("fa", 2, &samples, &buses);
        assert!(matches.contains(&"fast".to_string()));
    }

    #[test]
    fn test_get_completions_sample() {
        let samples = vec!["bd".to_string(), "bass".to_string(), "bend".to_string()];
        let buses = vec![];

        let matches = get_completions("s \"b", 4, &samples, &buses);
        assert_eq!(matches.len(), 3);
        assert!(matches.contains(&"bd".to_string()));
    }

    #[test]
    fn test_get_completions_bus() {
        let samples = vec![];
        let buses = vec!["drums".to_string(), "bass".to_string()];

        let matches = get_completions("s \"~dru", 7, &samples, &buses);
        assert_eq!(matches, vec!["~drums"]);
    }

    #[test]
    fn test_apply_completion_function() {
        let (new_line, new_cursor) = apply_completion("fa 2", 2, "fast");
        assert_eq!(new_line, "fast 2");
        assert_eq!(new_cursor, 4);
    }

    #[test]
    fn test_apply_completion_sample() {
        let (new_line, new_cursor) = apply_completion("s \"b", 4, "bd");
        assert_eq!(new_line, "s \"bd");
        assert_eq!(new_cursor, 5);
    }

    #[test]
    fn test_apply_completion_bus() {
        let (new_line, new_cursor) = apply_completion("s \"~drum", 8, "~drums");
        assert_eq!(new_line, "s \"~drums");
        assert_eq!(new_cursor, 9);
    }
}
```

### 2.2 State Machine

**File: `modal_editor/completion_state.rs`**
```rust
#[derive(Debug, Clone)]
pub struct CompletionState {
    pub visible: bool,
    pub matches: Vec<String>,
    pub selected_index: usize,
}

impl CompletionState {
    pub fn new() -> Self {
        Self {
            visible: false,
            matches: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn show(&mut self, matches: Vec<String>) {
        if matches.is_empty() {
            self.hide();
        } else {
            self.visible = true;
            self.matches = matches;
            self.selected_index = 0;
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.matches.clear();
        self.selected_index = 0;
    }

    pub fn move_down(&mut self) {
        if !self.matches.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.matches.len() - 1);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn selected(&self) -> Option<&str> {
        if self.visible && !self.matches.is_empty() {
            Some(&self.matches[self.selected_index])
        } else {
            None
        }
    }
}
```

**Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_starts_hidden() {
        let state = CompletionState::new();
        assert!(!state.visible);
        assert_eq!(state.matches.len(), 0);
    }

    #[test]
    fn test_show_completions() {
        let mut state = CompletionState::new();
        state.show(vec!["fast".to_string(), "filter".to_string()]);

        assert!(state.visible);
        assert_eq!(state.matches.len(), 2);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_navigation_down() {
        let mut state = CompletionState::new();
        state.show(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

        state.move_down();
        assert_eq!(state.selected_index, 1);

        state.move_down();
        assert_eq!(state.selected_index, 2);

        // Should stop at end
        state.move_down();
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_navigation_up() {
        let mut state = CompletionState::new();
        state.show(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        state.selected_index = 2;

        state.move_up();
        assert_eq!(state.selected_index, 1);

        state.move_up();
        assert_eq!(state.selected_index, 0);

        // Should stop at start
        state.move_up();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_selected_completion() {
        let mut state = CompletionState::new();
        state.show(vec!["fast".to_string(), "filter".to_string()]);

        assert_eq!(state.selected(), Some("fast"));

        state.move_down();
        assert_eq!(state.selected(), Some("filter"));
    }

    #[test]
    fn test_hide_clears_state() {
        let mut state = CompletionState::new();
        state.show(vec!["test".to_string()]);
        state.move_down();

        state.hide();

        assert!(!state.visible);
        assert_eq!(state.matches.len(), 0);
        assert_eq!(state.selected_index, 0);
    }
}
```

### 2.3 Integration into ModalEditor

```rust
// modal_editor/mod.rs
use completion::*;
use completion_state::CompletionState;

impl ModalEditor {
    fn handle_tab_key(&mut self) {
        let line = self.get_current_line();
        let cursor = self.cursor_col;

        let matches = get_completions(
            &line,
            cursor,
            &self.sample_names,
            &self.bus_names,
        );

        self.completion_state.show(matches);
    }

    fn handle_arrow_down(&mut self) {
        if self.completion_state.visible {
            self.completion_state.move_down();
        } else {
            // Normal cursor movement
        }
    }

    fn handle_enter(&mut self) {
        if let Some(completion) = self.completion_state.selected() {
            let line = self.get_current_line();
            let cursor = self.cursor_col;

            let (new_line, new_cursor) = apply_completion(
                &line,
                cursor,
                completion,
            );

            self.set_current_line(new_line);
            self.cursor_col = new_cursor;
            self.completion_state.hide();
        } else {
            // Normal enter behavior
        }
    }
}
```

## Part 3: Sample Discovery

**File: `modal_editor/sample_discovery.rs`**
```rust
use std::fs;
use std::path::PathBuf;

/// Discover all sample names from dirt-samples directories
pub fn discover_samples() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let base_path = PathBuf::from(home).join("dirt-samples");

    if !base_path.exists() {
        return Vec::new();
    }

    let mut samples = Vec::new();

    if let Ok(entries) = fs::read_dir(&base_path) {
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

    samples.sort();
    samples
}

/// Extract bus names from current editor content
pub fn extract_bus_names(content: &str) -> Vec<String> {
    let mut buses = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('~') {
            if let Some(colon_pos) = trimmed.find(':') {
                let name = &trimmed[1..colon_pos].trim();
                buses.push(name.to_string());
            }
        }
    }

    buses.sort();
    buses.dedup();
    buses
}
```

**Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bus_names() {
        let content = r#"
tempo: 0.5
~drums: s "bd sn"
~bass: saw 55
~lfo: sine 0.25
"#;
        let buses = extract_bus_names(content);
        assert_eq!(buses, vec!["bass", "drums", "lfo"]);
    }

    #[test]
    fn test_extract_bus_names_deduplicates() {
        let content = r#"
~drums: s "bd"
~drums: s "sn"
"#;
        let buses = extract_bus_names(content);
        assert_eq!(buses, vec!["drums"]);
    }
}
```

## Implementation Timeline

### Phase 1: Testing Infrastructure (Week 1)
- [x] Extract `highlight_line` to `highlighting.rs`
- [ ] Add 10+ tests for syntax highlighting
- [ ] Verify all tests pass in CI

### Phase 2: Completion Logic (Week 2)
- [ ] Implement `token.rs` with tests
- [ ] Implement `completion.rs` with tests
- [ ] Implement `completion_state.rs` with tests
- [ ] All tests passing before UI integration

### Phase 3: Sample Discovery (Week 3)
- [ ] Implement `sample_discovery.rs`
- [ ] Test with real dirt-samples directory
- [ ] Add tests with mock filesystem

### Phase 4: UI Integration (Week 4)
- [ ] Wire Tab key handler
- [ ] Wire arrow navigation
- [ ] Wire Enter to apply completion
- [ ] Add completion popup rendering
- [ ] Manual testing in live editor

### Phase 5: Polish (Week 5)
- [ ] Handle edge cases
- [ ] Add Escape to dismiss
- [ ] Performance optimization
- [ ] Documentation

## Success Criteria

### Must Have
- ✅ 90%+ test coverage for pure logic functions
- ✅ All tests pass in CI
- ✅ Tab completion works for functions
- ✅ Tab completion works for samples
- ✅ Tab completion works for buses
- ✅ Arrow keys navigate completions
- ✅ Enter applies selected completion
- ✅ Escape dismisses popup

### Nice to Have
- Fuzzy matching (not just prefix)
- Completion ranking by frequency
- Cache sample discovery results
- Visual indicator for completion type (sample/bus/function)

## Risk Mitigation

**Risk**: Changes break existing editor functionality
**Mitigation**: Pure function extraction means existing code unchanged until final integration

**Risk**: Tests don't catch real bugs
**Mitigation**: Property-based tests + extensive manual testing phase

**Risk**: Performance issues scanning samples
**Mitigation**: Cache results, lazy load, benchmark with large directories

## Testing Commands

```bash
# Run all modal editor tests
cargo test --lib modal_editor

# Run specific module tests
cargo test --lib modal_editor::highlighting
cargo test --lib modal_editor::completion
cargo test --lib modal_editor::token

# Run with coverage
cargo tarpaulin --lib --exclude-files "src/main.rs"

# Benchmark sample discovery
cargo bench --bench sample_discovery
```

## Conclusion

This strategy provides:
1. **High confidence** through comprehensive testing
2. **Safe refactoring** by extracting pure functions
3. **Clear milestones** with incremental progress
4. **Low risk** by keeping UI integration last

The pure function approach means ~90% of logic is tested in CI, while only the final UI wiring requires manual testing.
