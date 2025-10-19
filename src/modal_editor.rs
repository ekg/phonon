//! Modal live coding editor with terminal UI
//!
//! Provides a full-screen text editor for writing Phonon DSL code with
//! real-time audio generation triggered by Shift+Enter

use crate::live_engine::LiveEngine;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::fs;
use std::io;
use std::path::PathBuf;

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
    /// Flash highlight for evaluated chunk (start_line, end_line, frames_remaining)
    flash_highlight: Option<(usize, usize, u8)>,
}

impl ModalEditor {
    /// Create a new modal editor
    pub fn new(
        duration: f32,
        file_path: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let content = if let Some(ref path) = file_path {
            if path.exists() {
                fs::read_to_string(path)?
            } else {
                String::new()
            }
        } else {
            // Default starter template
            String::from("# Phonon Live Coding\n# C-x: Eval block | C-r: Reload all | C-h: Hush | C-s: Save | Alt-q: Quit\n\n# Example: Simple drum pattern\ntempo: 2.0\n~drums: s \"bd sn bd sn\"\nout: ~drums * 0.8\n")
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
            status_message:
                "🎵 Ready - C-x: eval block | C-r: reload all | C-h: hush".to_string(),
            is_playing: false,
            error_message: None,
            live_engine,
            flash_highlight: None,
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
    fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Decrement flash counter
            if let Some((start, end, frames)) = self.flash_highlight {
                if frames > 0 {
                    self.flash_highlight = Some((start, end, frames - 1));
                } else {
                    self.flash_highlight = None;
                }
            }

            terminal.draw(|f| self.ui(f))?;

            // Use poll with timeout to enable flash animation
            if event::poll(std::time::Duration::from_millis(50))? {
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
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key_event(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            // Quit with Alt+Q (Ctrl+Q conflicts with terminal flow control)
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::ALT) => KeyResult::Quit,

            // Ctrl+X: Evaluate current block (chunk)
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.eval_chunk();
                KeyResult::Continue
            }

            // Ctrl+R: Reload/eval entire session
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.eval_all();
                KeyResult::Continue
            }

            // Ctrl+H: Hush
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.hush();
                KeyResult::Continue
            }

            // Ctrl+S: Save
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyResult::Save,

            // Emacs-style movement keys
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_right(); // Forward
                KeyResult::Continue
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_left(); // Backward
                KeyResult::Continue
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_down(); // Next line
                KeyResult::Continue
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_up(); // Previous line
                KeyResult::Continue
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_line_start(); // Beginning of line
                KeyResult::Continue
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_line_end(); // End of line
                KeyResult::Continue
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.delete_char_forward(); // Delete forward
                KeyResult::Continue
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.kill_line(); // Kill to end of line
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
                Constraint::Min(3),    // Editor area
                Constraint::Length(3), // Status area
            ])
            .split(f.size());

        // Editor area with white borders
        let editor_block = Block::default()
            .title("Phonon Live Coding")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));

        let content_with_cursor = self.content_with_cursor();
        let paragraph = Paragraph::new(content_with_cursor)
            .block(editor_block)
            .wrap(Wrap { trim: false })
            .scroll((0, 0))
            .style(Style::default().fg(Color::White).bg(Color::Black));

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
            format!("❌ Error: {error}")
        } else if self.is_playing {
            "🔊 Playing...".to_string()
        } else {
            self.status_message.clone()
        };

        let help_text = "C-x: Eval block | C-r: Reload all | C-h: Hush | C-s: Save | Alt-q: Quit | Emacs keys: C-p/n/f/b/a/e";

        let status_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Status message
                Constraint::Length(1), // Help text
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

    /// Apply syntax highlighting to a line of Phonon code
    fn highlight_line(line: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut in_comment = false;

        let keywords = ["tempo", "out", "out1", "out2", "out3", "out4", "out5", "out6", "out7", "out8",
                       "s", "sine", "saw", "square", "tri", "lpf", "hpf", "bpf", "notch",
                       "reverb", "delay", "chorus", "bitcrush", "distortion",
                       "fast", "slow", "rev", "every", "euclid", "degrade", "degradeBy",
                       "stutter", "palindrome", "hush", "panic"];

        for (i, ch) in line.chars().enumerate() {
            // Comment detection
            if ch == '#' && !in_string {
                // Flush current token
                if !current.is_empty() {
                    let style = if keywords.contains(&current.as_str()) {
                        Style::default().fg(Color::Cyan)
                    } else if current.starts_with('~') {
                        Style::default().fg(Color::Magenta)
                    } else if current.chars().all(|c| c.is_ascii_digit() || c == '.') {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    spans.push(Span::styled(current.clone(), style));
                    current.clear();
                }
                // Rest of line is comment
                in_comment = true;
                current.push(ch);
                continue;
            }

            if in_comment {
                current.push(ch);
                continue;
            }

            // String detection
            if ch == '"' {
                if in_string {
                    current.push(ch);
                    spans.push(Span::styled(current.clone(), Style::default().fg(Color::Green)));
                    current.clear();
                    in_string = false;
                } else {
                    // Flush current token
                    if !current.is_empty() {
                        let style = if keywords.contains(&current.as_str()) {
                            Style::default().fg(Color::Cyan)
                        } else if current.starts_with('~') {
                            Style::default().fg(Color::Magenta)
                        } else if current.chars().all(|c| c.is_ascii_digit() || c == '.') {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        spans.push(Span::styled(current.clone(), style));
                        current.clear();
                    }
                    current.push(ch);
                    in_string = true;
                }
                continue;
            }

            if in_string {
                current.push(ch);
                continue;
            }

            // Operators and delimiters
            if "(){}[]:|$<>=+*-/,".contains(ch) {
                // Flush current token
                if !current.is_empty() {
                    let style = if keywords.contains(&current.as_str()) {
                        Style::default().fg(Color::Cyan)
                    } else if current.starts_with('~') {
                        Style::default().fg(Color::Magenta)
                    } else if current.chars().all(|c| c.is_ascii_digit() || c == '.') {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    spans.push(Span::styled(current.clone(), style));
                    current.clear();
                }
                // Operator
                spans.push(Span::styled(ch.to_string(), Style::default().fg(Color::Red)));
                continue;
            }

            // Whitespace
            if ch.is_whitespace() {
                // Flush current token
                if !current.is_empty() {
                    let style = if keywords.contains(&current.as_str()) {
                        Style::default().fg(Color::Cyan)
                    } else if current.starts_with('~') {
                        Style::default().fg(Color::Magenta)
                    } else if current.chars().all(|c| c.is_ascii_digit() || c == '.') {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    spans.push(Span::styled(current.clone(), style));
                    current.clear();
                }
                spans.push(Span::raw(ch.to_string()));
                continue;
            }

            current.push(ch);
        }

        // Flush remaining
        if !current.is_empty() {
            let style = if in_comment {
                Style::default().fg(Color::DarkGray)
            } else if in_string {
                Style::default().fg(Color::Green)
            } else if keywords.contains(&current.as_str()) {
                Style::default().fg(Color::Cyan)
            } else if current.starts_with('~') {
                Style::default().fg(Color::Magenta)
            } else if current.chars().all(|c| c.is_ascii_digit() || c == '.') {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            spans.push(Span::styled(current, style));
        }

        if spans.is_empty() {
            spans.push(Span::raw(" "));
        }

        spans
    }

    /// Get content with cursor indicator and syntax highlighting
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

        // Check if we're flashing a chunk
        // Smooth pop-to-fade: white flash then smooth fade over 0.5s
        let (flash_start, flash_end, flash_color) = if let Some((start, end, frames)) = self.flash_highlight {
            if frames > 8 {
                // Full white pop (frames 10-9 = 100ms)
                (start, end, Color::Rgb(255, 255, 255))
            } else if frames > 0 {
                // Smooth fade (frames 8-1 = 400ms)
                let fade_progress = (8 - frames) as f32 / 8.0; // 0.0 = white, 1.0 = black
                let brightness = (255.0 * (1.0 - fade_progress)) as u8;
                (start, end, Color::Rgb(brightness, brightness, brightness))
            } else {
                (usize::MAX, usize::MAX, Color::Black)
            }
        } else {
            (usize::MAX, usize::MAX, Color::Black)
        };

        // Render lines with cursor, flash highlight, and syntax highlighting
        for (line_idx, line_text) in text_lines.iter().enumerate() {
            let is_flashing = line_idx >= flash_start && line_idx <= flash_end;

            if line_idx == cursor_line {
                // Line with cursor - needs special handling for cursor position
                let mut spans = Vec::new();

                if line_text.is_empty() {
                    // Empty line - show cursor block
                    if is_flashing {
                        // Flash background with cursor
                        spans.push(Span::styled("█", Style::default().fg(Color::White).bg(flash_color)));
                    } else {
                        spans.push(Span::styled("█", Style::default().fg(Color::White)));
                    }
                } else if cursor_col < line_text.len() {
                    // Cursor in middle of line - apply syntax highlighting with cursor
                    let before = &line_text[..cursor_col];
                    let at_cursor = line_text.chars().nth(cursor_col).unwrap().to_string();
                    let after = &line_text[cursor_col + 1..];

                    // Highlight before cursor
                    if !before.is_empty() {
                        let mut before_spans = Self::highlight_line(before);
                        if is_flashing {
                            // Add flash background to all spans
                            for span in &mut before_spans {
                                span.style = span.style.bg(flash_color).fg(Color::Black);
                            }
                        }
                        spans.append(&mut before_spans);
                    }

                    // Cursor character with white background
                    spans.push(Span::styled(
                        at_cursor,
                        Style::default().bg(Color::White).fg(Color::Black),
                    ));

                    // Highlight after cursor
                    if !after.is_empty() {
                        let mut after_spans = Self::highlight_line(after);
                        if is_flashing {
                            // Add flash background to all spans
                            for span in &mut after_spans {
                                span.style = span.style.bg(flash_color).fg(Color::Black);
                            }
                        }
                        spans.append(&mut after_spans);
                    }
                } else {
                    // Cursor at end of line
                    let mut highlighted = Self::highlight_line(line_text);
                    if is_flashing {
                        // Add flash background to all spans
                        for span in &mut highlighted {
                            span.style = span.style.bg(flash_color).fg(Color::Black);
                        }
                    }
                    spans.append(&mut highlighted);
                    // Cursor block at end
                    spans.push(Span::styled("█", Style::default().fg(Color::White)));
                }
                lines.push(Line::from(spans));
            } else {
                // Regular line - apply syntax highlighting
                if line_text.is_empty() {
                    if is_flashing {
                        lines.push(Line::from(Span::styled(" ", Style::default().bg(flash_color))));
                    } else {
                        lines.push(Line::from(Span::raw(" "))); // Ensure empty lines take space
                    }
                } else {
                    let mut spans = Self::highlight_line(line_text);
                    if is_flashing {
                        // Add flash background to all spans
                        for span in &mut spans {
                            span.style = span.style.bg(flash_color).fg(Color::Black);
                        }
                    }
                    lines.push(Line::from(spans));
                }
            }
        }

        // Handle cursor at very end of empty content
        if lines.is_empty() && self.cursor_pos == 0 {
            // Show cursor block for empty file
            lines.push(Line::from(Span::styled(
                "█",
                Style::default().fg(Color::White),
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
            let char_start = self
                .content
                .char_indices()
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
                self.error_message = Some(format!("Failed to load: {e}"));
            } else {
                self.status_message = "✅ Pattern reloaded!".to_string();
            }
        } else {
            self.error_message = Some("Live engine not running".to_string());
        }
    }

    /// Evaluate current chunk (paragraph) - Ctrl-X
    /// A chunk is text separated by blank lines
    ///
    /// Note: We send the FULL session content to the engine, not just the chunk.
    /// The flash highlight shows which chunk you evaluated for visual feedback.
    fn eval_chunk(&mut self) {
        let chunk = self.get_current_chunk();
        if chunk.trim().is_empty() {
            self.status_message = "⚠️  Empty chunk".to_string();
            return;
        }

        // Get chunk boundaries for flash highlight
        let (start_line, end_line) = self.get_current_chunk_lines();

        if let Some(ref engine) = self.live_engine {
            self.error_message = None;
            self.status_message = format!("🔄 Evaluating chunk ({} chars)...", chunk.len());

            // IMPORTANT: Send the full session content, not just the chunk!
            // This ensures all buses, tempo, and output assignments are preserved.
            if let Err(e) = engine.load_code(&self.content) {
                self.error_message = Some(format!("Eval failed: {e}"));
            } else {
                self.status_message = "✅ Chunk evaluated!".to_string();
                // Flash the evaluated chunk: 10 frames = 500ms (pop + fade)
                self.flash_highlight = Some((start_line, end_line, 10));
            }
        } else {
            self.error_message = Some("Live engine not running".to_string());
        }
    }

    /// Evaluate entire session - Ctrl-R (Reload)
    fn eval_all(&mut self) {
        if self.content.trim().is_empty() {
            self.status_message = "⚠️  Empty session".to_string();
            return;
        }

        if let Some(ref engine) = self.live_engine {
            self.error_message = None;
            self.status_message = "🔄 Reloading entire session...".to_string();

            if let Err(e) = engine.load_code(&self.content) {
                self.error_message = Some(format!("Reload failed: {e}"));
            } else {
                self.status_message = "✅ Session reloaded!".to_string();
            }
        } else {
            self.error_message = Some("Live engine not running".to_string());
        }
    }

    /// Get the current chunk (paragraph) around cursor
    /// A chunk is text between blank lines
    fn get_current_chunk(&self) -> String {
        let lines: Vec<&str> = self.content.split('\n').collect();
        let mut current_pos = 0;
        let mut cursor_line_idx = 0;

        // Find cursor line
        for (idx, line) in lines.iter().enumerate() {
            if current_pos + line.len() >= self.cursor_pos {
                cursor_line_idx = idx;
                break;
            }
            current_pos += line.len() + 1;
        }

        // Find chunk boundaries (blank lines)
        let mut start_idx = cursor_line_idx;
        let mut end_idx = cursor_line_idx;

        // Search backwards for blank line or start
        while start_idx > 0 && !lines[start_idx - 1].trim().is_empty() {
            start_idx -= 1;
        }

        // Search forwards for blank line or end
        while end_idx < lines.len() - 1 && !lines[end_idx + 1].trim().is_empty() {
            end_idx += 1;
        }

        // Extract chunk
        lines[start_idx..=end_idx].join("\n")
    }

    /// Get the current chunk line boundaries (for flash highlight)
    fn get_current_chunk_lines(&self) -> (usize, usize) {
        let lines: Vec<&str> = self.content.split('\n').collect();
        let mut current_pos = 0;
        let mut cursor_line_idx = 0;

        // Find cursor line
        for (idx, line) in lines.iter().enumerate() {
            if current_pos + line.len() >= self.cursor_pos {
                cursor_line_idx = idx;
                break;
            }
            current_pos += line.len() + 1;
        }

        // Find chunk boundaries (blank lines)
        let mut start_idx = cursor_line_idx;
        let mut end_idx = cursor_line_idx;

        // Search backwards for blank line or start
        while start_idx > 0 && !lines[start_idx - 1].trim().is_empty() {
            start_idx -= 1;
        }

        // Search forwards for blank line or end
        while end_idx < lines.len() - 1 && !lines[end_idx + 1].trim().is_empty() {
            end_idx += 1;
        }

        (start_idx, end_idx)
    }

    /// Hush - silence all sound
    fn hush(&mut self) {
        if let Some(ref engine) = self.live_engine {
            if let Err(e) = engine.hush() {
                self.error_message = Some(format!("Hush failed: {e}"));
            } else {
                self.status_message = "🔇 Hushed - C-r to reload".to_string();
            }
        }
    }

    /// Panic - stop everything
    fn panic(&mut self) {
        if let Some(ref engine) = self.live_engine {
            if let Err(e) = engine.panic() {
                self.error_message = Some(format!("Panic failed: {e}"));
            } else {
                self.status_message = "🚨 PANIC! All stopped - C-r to restart".to_string();
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
