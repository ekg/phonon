//! Modal live coding editor with terminal UI
//! 
//! Provides a full-screen text editor for writing Phonon DSL code with
//! real-time audio generation triggered by Shift+Enter

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap, Clear},
    Frame, Terminal,
};
use std::path::PathBuf;
use std::fs;
use std::io;
use crate::live_engine::LiveEngine;

/// Modal live coding editor state
pub struct ModalEditor {
    /// Current text content
    content: String,
    /// Cursor position in the content
    cursor_pos: usize,
    /// Duration for audio renders (cycle length)
    duration: f32,
    /// Current file path (if any)
    file_path: Option<PathBuf>,
    /// Status message to display
    status_message: String,
    /// Whether we're currently playing
    is_playing: bool,
    /// Error message (if any)
    error_message: Option<String>,
    /// Live audio engine
    live_engine: Option<LiveEngine>,
}

impl ModalEditor {
    /// Create a new modal editor
    pub fn new(duration: f32, file_path: Option<PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = if let Some(ref path) = file_path {
            if path.exists() {
                fs::read_to_string(path)?
            } else {
                String::new()
            }
        } else {
            // Default starter template
            String::from("# Phonon Live Coding\n# Press Ctrl+X to play, Ctrl+S to save, Ctrl+Q to quit\n# Emacs keys: C-f/b (char), C-n/p (line), C-a/e (line start/end)\n\n# Example: Simple drum pattern\n~drums: s \"bd sn bd sn\"\nout: ~drums >> mul 0.5\n")
        };

        // Start the live audio engine
        let live_engine = LiveEngine::new(44100.0, duration).ok();
        
        // Load the initial pattern into the engine
        if let Some(ref engine) = live_engine {
            let _ = engine.load_code(&content);
        }
        
        Ok(Self {
            cursor_pos: content.len(),
            content,
            duration,
            file_path,
            status_message: "🎵 Live Engine Running - Ctrl+X: reload | Ctrl+H: hush | Ctrl+P: panic".to_string(),
            is_playing: false,
            error_message: None,
            live_engine,
        })
    }

    /// Run the modal editor
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    /// Main application loop
    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                match self.handle_key_event(key) {
                    KeyResult::Continue => continue,
                    KeyResult::Quit => break,
                    KeyResult::Play => {
                        self.play_code();
                    }
                    KeyResult::Save => {
                        self.save_file()?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key_event(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            // Control key combinations
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                KeyResult::Quit
            }
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                KeyResult::Play  // Ctrl+X for eXecute/reload
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.hush();
                KeyResult::Continue
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                KeyResult::Save
            }
            // Emacs-style movement
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_right();  // Forward
                KeyResult::Continue
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_left();   // Backward
                KeyResult::Continue
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_down();   // Next line
                KeyResult::Continue
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_up();     // Previous line
                KeyResult::Continue
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_line_start();  // Beginning of line
                KeyResult::Continue
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_line_end();    // End of line
                KeyResult::Continue
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.delete_char_forward();     // Delete forward
                KeyResult::Continue
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.kill_line();               // Kill to end of line
                KeyResult::Continue
            }
            // Regular character input
            KeyCode::Char(c) => {
                self.insert_char(c);
                KeyResult::Continue
            }
            KeyCode::Enter => {
                self.insert_char('\n');
                KeyResult::Continue
            }
            KeyCode::Backspace => {
                self.delete_char();
                KeyResult::Continue
            }
            // Arrow keys still work
            KeyCode::Left => {
                self.move_cursor_left();
                KeyResult::Continue
            }
            KeyCode::Right => {
                self.move_cursor_right();
                KeyResult::Continue
            }
            KeyCode::Up => {
                self.move_cursor_up();
                KeyResult::Continue
            }
            KeyCode::Down => {
                self.move_cursor_down();
                KeyResult::Continue
            }
            KeyCode::Home => {
                self.move_cursor_line_start();
                KeyResult::Continue
            }
            KeyCode::End => {
                self.move_cursor_line_end();
                KeyResult::Continue
            }
            _ => KeyResult::Continue,
        }
    }

    /// Render the UI
    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),      // Editor area
                Constraint::Length(3),   // Status area
            ])
            .split(f.size());

        // Editor area
        let editor_block = Block::default()
            .title("Phonon Live Coding Editor")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let content_with_cursor = self.content_with_cursor();
        let paragraph = Paragraph::new(content_with_cursor)
            .block(editor_block)
            .wrap(Wrap { trim: false })
            .scroll((0, 0));

        f.render_widget(paragraph, chunks[0]);

        // Status area
        let status_style = if self.error_message.is_some() {
            Style::default().fg(Color::Red)
        } else if self.is_playing {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let status_text = if let Some(ref error) = self.error_message {
            format!("❌ Error: {}", error)
        } else if self.is_playing {
            "🔊 Playing...".to_string()
        } else {
            self.status_message.clone()
        };

        let help_text = "C-x: Reload | C-h: Hush | C-s: Save | C-q: Quit | C-p/n: ↑↓ | C-f/b: ←→";
        
        let status_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),   // Status message
                Constraint::Length(1),   // Help text
            ])
            .split(chunks[1]);

        let status_paragraph = Paragraph::new(status_text)
            .style(status_style)
            .alignment(Alignment::Left);

        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(status_paragraph, status_chunks[0]);
        f.render_widget(help_paragraph, status_chunks[1]);
    }

    /// Get content with cursor indicator
    fn content_with_cursor(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        let text_lines: Vec<&str> = self.content.split('\n').collect();
        
        let mut current_pos = 0;
        let mut cursor_line = 0;
        let mut cursor_col = 0;

        // Find cursor position in terms of line/column
        for (line_idx, line) in text_lines.iter().enumerate() {
            if current_pos + line.len() >= self.cursor_pos {
                cursor_line = line_idx;
                cursor_col = self.cursor_pos - current_pos;
                break;
            }
            current_pos += line.len() + 1; // +1 for newline
        }

        // Render lines with cursor
        for (line_idx, line_text) in text_lines.iter().enumerate() {
            if line_idx == cursor_line {
                // Line with cursor
                let mut spans = Vec::new();
                
                if line_text.is_empty() {
                    // Empty line - just show cursor block
                    spans.push(Span::styled(
                        " ",
                        Style::default().bg(Color::White)
                    ));
                } else if cursor_col < line_text.len() {
                    // Cursor in middle of line
                    if cursor_col > 0 {
                        spans.push(Span::raw(line_text[..cursor_col].to_string()));
                    }
                    spans.push(Span::styled(
                        line_text.chars().nth(cursor_col).unwrap().to_string(),
                        Style::default().bg(Color::White).fg(Color::Black)
                    ));
                    if cursor_col + 1 < line_text.len() {
                        spans.push(Span::raw(line_text[cursor_col + 1..].to_string()));
                    }
                } else {
                    // Cursor at end of line
                    spans.push(Span::raw(line_text.to_string()));
                    spans.push(Span::styled(
                        " ",
                        Style::default().bg(Color::White)
                    ));
                }
                lines.push(Line::from(spans));
            } else {
                // Regular line (including empty lines)
                if line_text.is_empty() {
                    lines.push(Line::from(Span::raw(" "))); // Ensure empty lines take space
                } else {
                    lines.push(Line::from(Span::raw(line_text.to_string())));
                }
            }
        }

        // Handle cursor at very end of empty content
        if lines.is_empty() && self.cursor_pos == 0 {
            lines.push(Line::from(Span::styled(
                " ",
                Style::default().bg(Color::White)
            )));
        }

        lines
    }

    /// Insert character at cursor position
    fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.error_message = None;
    }

    /// Delete character before cursor
    fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            let char_start = self.content.char_indices()
                .nth(self.cursor_pos.saturating_sub(1))
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.content.remove(char_start);
            self.cursor_pos = char_start;
        }
        self.error_message = None;
    }

    /// Delete character forward (Ctrl+D)
    fn delete_char_forward(&mut self) {
        if self.cursor_pos < self.content.len() {
            self.content.remove(self.cursor_pos);
        }
        self.error_message = None;
    }

    /// Kill to end of line (Ctrl+K)
    fn kill_line(&mut self) {
        let lines: Vec<&str> = self.content.split('\n').collect();
        let mut current_pos = 0;
        
        for line in lines.iter() {
            if current_pos + line.len() >= self.cursor_pos {
                // Found current line
                let line_start = current_pos;
                let line_end = current_pos + line.len();
                
                if self.cursor_pos < line_end {
                    // Remove from cursor to end of line
                    self.content.drain(self.cursor_pos..line_end);
                }
                break;
            }
            current_pos += line.len() + 1; // +1 for newline
        }
        self.error_message = None;
    }

    /// Move cursor left
    fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right  
    fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.content.len() {
            self.cursor_pos += 1;
        }
    }

    /// Move cursor up one line
    fn move_cursor_up(&mut self) {
        let lines: Vec<&str> = self.content.split('\n').collect();
        let mut current_pos = 0;
        let mut line_idx = 0;
        let mut col_in_line = 0;

        // Find current line and column
        for (idx, line) in lines.iter().enumerate() {
            if current_pos + line.len() >= self.cursor_pos {
                line_idx = idx;
                col_in_line = self.cursor_pos - current_pos;
                break;
            }
            current_pos += line.len() + 1;
        }

        if line_idx > 0 {
            // Move to previous line
            let prev_line = lines[line_idx - 1];
            let new_col = col_in_line.min(prev_line.len());
            
            // Calculate new cursor position
            let mut new_pos = 0;
            for i in 0..line_idx - 1 {
                new_pos += lines[i].len() + 1;
            }
            new_pos += new_col;
            
            self.cursor_pos = new_pos;
        }
    }

    /// Move cursor down one line
    fn move_cursor_down(&mut self) {
        let lines: Vec<&str> = self.content.split('\n').collect();
        let mut current_pos = 0;
        let mut line_idx = 0;
        let mut col_in_line = 0;

        // Find current line and column
        for (idx, line) in lines.iter().enumerate() {
            if current_pos + line.len() >= self.cursor_pos {
                line_idx = idx;
                col_in_line = self.cursor_pos - current_pos;
                break;
            }
            current_pos += line.len() + 1;
        }

        if line_idx < lines.len() - 1 {
            // Move to next line
            let next_line = lines[line_idx + 1];
            let new_col = col_in_line.min(next_line.len());
            
            // Calculate new cursor position
            let mut new_pos = 0;
            for i in 0..line_idx + 1 {
                new_pos += lines[i].len() + 1;
            }
            new_pos += new_col;
            
            self.cursor_pos = new_pos.min(self.content.len());
        }
    }

    /// Move cursor to start of current line
    fn move_cursor_line_start(&mut self) {
        let lines: Vec<&str> = self.content.split('\n').collect();
        let mut current_pos = 0;

        for line in lines.iter() {
            if current_pos + line.len() >= self.cursor_pos {
                self.cursor_pos = current_pos;
                break;
            }
            current_pos += line.len() + 1;
        }
    }

    /// Move cursor to end of current line
    fn move_cursor_line_end(&mut self) {
        let lines: Vec<&str> = self.content.split('\n').collect();
        let mut current_pos = 0;

        for line in lines.iter() {
            if current_pos + line.len() >= self.cursor_pos {
                self.cursor_pos = current_pos + line.len();
                break;
            }
            current_pos += line.len() + 1;
        }
    }

    /// Reload pattern into live engine
    fn play_code(&mut self) {
        if let Some(ref engine) = self.live_engine {
            self.error_message = None;
            self.status_message = "🔄 Reloading pattern...".to_string();
            
            // Send the code to the live engine
            if let Err(e) = engine.load_code(&self.content) {
                self.error_message = Some(format!("Failed to load: {}", e));
            } else {
                self.status_message = "✅ Pattern reloaded!".to_string();
            }
        } else {
            self.error_message = Some("Live engine not running".to_string());
        }
    }
    
    /// Hush - silence all sound
    fn hush(&mut self) {
        if let Some(ref engine) = self.live_engine {
            if let Err(e) = engine.hush() {
                self.error_message = Some(format!("Hush failed: {}", e));
            } else {
                self.status_message = "🔇 Hushed - Ctrl+X to resume".to_string();
            }
        }
    }
    
    /// Panic - stop everything
    fn panic(&mut self) {
        if let Some(ref engine) = self.live_engine {
            if let Err(e) = engine.panic() {
                self.error_message = Some(format!("Panic failed: {}", e));
            } else {
                self.status_message = "🚨 PANIC! All stopped - Ctrl+X to restart".to_string();
            }
        }
    }

    /// Save the current file
    fn save_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref path) = self.file_path {
            fs::write(path, &self.content)?;
            self.status_message = format!("💾 Saved to {}", path.display());
        } else {
            // Prompt for filename (simplified - just use a default)
            let default_path = PathBuf::from("untitled.phonon");
            fs::write(&default_path, &self.content)?;
            self.file_path = Some(default_path.clone());
            self.status_message = format!("💾 Saved to {}", default_path.display());
        }
        self.error_message = None;
        Ok(())
    }
}

/// Key event result
enum KeyResult {
    Continue,
    Quit,
    Play,
    Save,
}