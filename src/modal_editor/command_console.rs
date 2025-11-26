//! Command console for help and discovery
//!
//! Provides a searchable help system accessible via Alt+/

use super::completion::*;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Command console state
pub struct CommandConsole {
    /// Whether the console is visible
    visible: bool,
    /// Command input buffer
    input: String,
    /// Cursor position in input
    cursor_pos: usize,
    /// Command results/output
    output: Vec<String>,
}

impl CommandConsole {
    /// Create a new command console
    pub fn new() -> Self {
        Self {
            visible: false,
            input: String::new(),
            cursor_pos: 0,
            output: vec!["Command console - type /help for help".to_string()],
        }
    }

    /// Toggle console visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            // Clear input when opening
            self.input.clear();
            self.cursor_pos = 0;
        }
    }

    /// Check if console is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Hide the console
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Insert character into input
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    /// Delete character before cursor
    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            let char_start = self
                .input
                .char_indices()
                .nth(self.cursor_pos.saturating_sub(1))
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(char_start);
            self.cursor_pos = char_start;
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }

    /// Execute the current command
    pub fn execute_command(&mut self) {
        let command = self.input.trim();
        self.output.clear();

        if command.is_empty() {
            return;
        }

        // Parse command
        let parts: Vec<&str> = command.split_whitespace().collect();
        let cmd = parts[0];

        match cmd {
            "/help" => {
                if parts.len() > 1 {
                    // Show help for specific function
                    let func_name = parts[1];
                    if let Some(metadata) = FUNCTION_METADATA.get(func_name) {
                        self.show_function_help(metadata);
                    } else {
                        self.output
                            .push(format!("Unknown function: {}", func_name));
                        self.output
                            .push("Type /functions to see all functions".to_string());
                    }
                } else {
                    // Show general help
                    self.show_general_help();
                }
            }

            "/functions" => {
                if parts.len() > 1 {
                    // Filter by category
                    let category = parts[1];
                    let funcs = functions_by_category(category);
                    if funcs.is_empty() {
                        self.output
                            .push(format!("No functions in category: {}", category));
                        self.output.push("Categories: Filters, Envelopes, Effects, Patterns, Transforms".to_string());
                    } else {
                        self.output
                            .push(format!("Functions in category '{}':", category));
                        for func in funcs {
                            self.output
                                .push(format!("  {} - {}", func.name, func.description));
                        }
                    }
                } else {
                    // Show all functions grouped by category
                    self.show_all_functions();
                }
            }

            "/search" => {
                if parts.len() > 1 {
                    let query = parts[1..].join(" ");
                    let results = search_functions(&query);
                    if results.is_empty() {
                        self.output.push(format!("No results for: {}", query));
                    } else {
                        self.output
                            .push(format!("Search results for '{}':", query));
                        for func in results {
                            self.output.push(format!(
                                "  {} ({}) - {}",
                                func.name, func.category, func.description
                            ));
                        }
                    }
                } else {
                    self.output.push("Usage: /search <query>".to_string());
                }
            }

            "/params" => {
                if parts.len() > 1 {
                    let func_name = parts[1];
                    if let Some(metadata) = FUNCTION_METADATA.get(func_name) {
                        self.show_function_params(metadata);
                    } else {
                        self.output
                            .push(format!("Unknown function: {}", func_name));
                    }
                } else {
                    self.output.push("Usage: /params <function>".to_string());
                }
            }

            "/categories" => {
                self.output.push("Function categories:".to_string());
                self.output.push("  Filters - lpf, hpf, bpf, notch".to_string());
                self.output.push("  Envelopes - adsr, ad, asr".to_string());
                self.output.push("  Effects - reverb, chorus, delay, distort".to_string());
                self.output.push("  Patterns - s (sample trigger)".to_string());
                self.output
                    .push("  Transforms - fast, slow, every, rev".to_string());
                self.output.push("".to_string());
                self.output.push("Usage: /functions <category>".to_string());
            }

            _ => {
                self.output
                    .push(format!("Unknown command: {}", cmd));
                self.output.push("Available commands:".to_string());
                self.output.push("  /help [function]".to_string());
                self.output.push("  /functions [category]".to_string());
                self.output.push("  /search <query>".to_string());
                self.output.push("  /params <function>".to_string());
                self.output.push("  /categories".to_string());
            }
        }

        // Clear input after execution
        self.input.clear();
        self.cursor_pos = 0;
    }

    /// Show general help
    fn show_general_help(&mut self) {
        self.output.push("Phonon Command Console".to_string());
        self.output.push("".to_string());
        self.output.push("Commands:".to_string());
        self.output
            .push("  /help [function]     - Show help for function".to_string());
        self.output
            .push("  /functions [cat]     - List all functions (optionally by category)".to_string());
        self.output
            .push("  /search <query>      - Search functions by name/description".to_string());
        self.output
            .push("  /params <function>   - Show parameters for function".to_string());
        self.output
            .push("  /categories          - List all categories".to_string());
        self.output.push("".to_string());
        self.output.push("Examples:".to_string());
        self.output.push("  /help lpf".to_string());
        self.output.push("  /functions Filters".to_string());
        self.output.push("  /search reverb".to_string());
        self.output.push("  /params adsr".to_string());
        self.output.push("".to_string());
        self.output.push("MIDI Input:".to_string());
        self.output.push("  Alt+M  - Connect to MIDI device (cycle through)".to_string());
        self.output.push("  Alt+R  - Start/stop MIDI recording".to_string());
        self.output.push("  Alt+I  - Insert recorded pattern (note names)".to_string());
        self.output.push("  Alt+N  - Insert recorded pattern (n-offsets from lowest)".to_string());
        self.output.push("  Alt+V  - Insert recorded velocities (as gain pattern)".to_string());
        self.output.push("".to_string());
        self.output.push("Tip: If recorded over N cycles, use $ slow N to fit pattern".to_string());
        self.output.push("".to_string());
        self.output.push("Press Esc or Alt+/ to close".to_string());
    }

    /// Show help for a specific function
    fn show_function_help(&mut self, metadata: &FunctionMetadata) {
        self.output
            .push(format!("{} ({})", metadata.name, metadata.category));
        self.output.push(metadata.description.to_string());
        self.output.push("".to_string());

        if !metadata.params.is_empty() {
            self.output.push("Parameters:".to_string());
            for param in &metadata.params {
                let optional_str = if param.optional {
                    if let Some(default) = param.default {
                        format!(" (optional, default: {})", default)
                    } else {
                        " (optional)".to_string()
                    }
                } else {
                    " (required)".to_string()
                };

                self.output.push(format!(
                    "  :{} {}{} - {}",
                    param.name, param.param_type, optional_str, param.description
                ));
            }
            self.output.push("".to_string());
        }

        self.output.push("Example:".to_string());
        self.output.push(format!("  {}", metadata.example));
    }

    /// Show parameters for a function
    fn show_function_params(&mut self, metadata: &FunctionMetadata) {
        self.output.push(format!("{} parameters:", metadata.name));
        self.output.push("".to_string());

        for (i, param) in metadata.params.iter().enumerate() {
            let position = format!("Position {}", i);
            let keyword = format!(":{}", param.name);
            let optional_str = if param.optional {
                if let Some(default) = param.default {
                    format!("optional (default: {})", default)
                } else {
                    "optional".to_string()
                }
            } else {
                "required".to_string()
            };

            self.output
                .push(format!("  {} / {}", position, keyword));
            self.output
                .push(format!("    Type: {} ({})", param.param_type, optional_str));
            self.output
                .push(format!("    {}", param.description));
            self.output.push("".to_string());
        }

        self.output.push("Usage examples:".to_string());
        self.output.push(format!("  {}", metadata.example));
    }

    /// Show all functions grouped by category
    fn show_all_functions(&mut self) {
        let categories = vec!["Filters", "Envelopes", "Effects", "Patterns", "Transforms"];

        self.output.push("All Phonon Functions".to_string());
        self.output.push("".to_string());

        for category in categories {
            let funcs = functions_by_category(category);
            if !funcs.is_empty() {
                self.output.push(format!("{}:", category));
                for func in funcs {
                    self.output
                        .push(format!("  {} - {}", func.name, func.description));
                }
                self.output.push("".to_string());
            }
        }

        self.output
            .push("Type /help <function> for details".to_string());
    }

    /// Render the console UI
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Split into input area and output area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Output area
                Constraint::Length(3), // Input area
            ])
            .split(area);

        // Render output
        let output_block = Block::default()
            .title("Command Output")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let output_lines: Vec<Line> = self
            .output
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect();

        let output_paragraph = Paragraph::new(output_lines)
            .block(output_block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));

        f.render_widget(output_paragraph, chunks[0]);

        // Render input with cursor
        let input_block = Block::default()
            .title("Command (Esc to close)")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));

        // Create input display with cursor
        let mut input_display = self.input.clone();
        if self.cursor_pos < input_display.len() {
            // Cursor in middle - highlight character
            input_display.insert_str(self.cursor_pos, "█");
        } else {
            // Cursor at end
            input_display.push('█');
        }

        let input_paragraph = Paragraph::new(input_display)
            .block(input_block)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left);

        f.render_widget(input_paragraph, chunks[1]);
    }
}
