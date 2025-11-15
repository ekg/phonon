#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Modal live coding editor with terminal UI
//!
//! Provides a full-screen text editor for writing Phonon DSL code with
//! real-time audio generation using ring buffer architecture for parallel synthesis

mod command_console;
mod completion;
mod highlighting;

use command_console::CommandConsole;
use highlighting::highlight_line;

use crate::compositional_compiler::compile_program;
use crate::compositional_parser::parse_program;
use crate::unified_graph::UnifiedSignalGraph;
use arc_swap::ArcSwap;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration as StdDuration;

// Newtype wrapper to impl Send+Sync for RefCell<UnifiedSignalGraph>
// SAFETY: Each GraphCell instance is only accessed by one thread at a time.
struct GraphCell(RefCell<UnifiedSignalGraph>);
unsafe impl Send for GraphCell {}
unsafe impl Sync for GraphCell {}

/// Modal live coding editor state
pub struct ModalEditor {
    /// Current text content
    content: String,
    /// Cursor position in the content
    cursor_pos: usize,
    /// Current file path (if any)
    file_path: Option<PathBuf>,
    /// Status message to display
    status_message: String,
    /// Whether we're currently playing
    is_playing: bool,
    /// Error message (if any)
    error_message: Option<String>,
    /// Shared audio graph (lock-free with ring buffer)
    graph: Arc<ArcSwap<Option<GraphCell>>>,
    /// Audio stream (kept alive)
    _stream: cpal::Stream,
    /// Sample rate
    sample_rate: f32,
    /// Flash highlight for evaluated chunk (start_line, end_line, frames_remaining)
    flash_highlight: Option<(usize, usize, u8)>,
    /// Kill buffer for Emacs-style cut/yank
    kill_buffer: String,
    /// Undo stack (content, cursor_pos)
    undo_stack: Vec<(String, usize)>,
    /// Redo stack (content, cursor_pos)
    redo_stack: Vec<(String, usize)>,
    /// Console messages for display
    console_messages: Vec<String>,
    /// Tab completion state
    completion_state: completion::CompletionState,
    /// Available sample names from ~/dirt-samples/
    sample_names: Vec<String>,
    /// Available bus names from current content
    bus_names: Vec<String>,
    /// Command console for help and discovery
    command_console: CommandConsole,
}

impl ModalEditor {
    /// Create a new modal editor
    pub fn new(
        _duration: f32, // Deprecated parameter, kept for API compatibility
        file_path: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Suppress ALSA error messages that would break the TUI
        // ALSA lib prints directly to stderr from C code, which we can't intercept in Rust
        // Redirect stderr to log file to prevent TUI corruption
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            if let Ok(log_file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/phonon_audio_errors.log")
            {
                unsafe {
                    libc::dup2(log_file.as_raw_fd(), libc::STDERR_FILENO);
                }
            }
        }

        // Get audio device
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let default_config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default config: {}", e))?;

        let sample_rate = default_config.sample_rate().0 as f32;
        let channels = default_config.channels() as usize;
        let sample_format = default_config.sample_format();

        // Use default buffer size (ring buffer handles buffering)
        let config: cpal::StreamConfig = default_config.into();

        eprintln!("🎵 Audio: {} Hz, {} channels",
                 sample_rate as u32, channels);
        eprintln!("🔧 Using ring buffer architecture for parallel synthesis");

        // Graph for background synthesis thread (lock-free swap)
        let graph = Arc::new(ArcSwap::from_pointee(None::<GraphCell>));

        // Ring buffer: background synth writes, audio callback reads
        // Size: 1 second of audio = smooth playback even if synth lags briefly
        let ring_buffer_size = sample_rate as usize;
        let ring = HeapRb::<f32>::new(ring_buffer_size);
        let (mut ring_producer, mut ring_consumer) = ring.split();

        // Background synthesis thread: continuously renders samples into ring buffer
        let graph_clone_synth = Arc::clone(&graph);
        thread::spawn(move || {
            let mut buffer = [0.0f32; 512]; // Render in chunks of 512 samples

            loop {
                // Check if we have space in ring buffer
                let space = ring_producer.vacant_len();

                if space >= buffer.len() {
                    // Render a chunk of audio
                    let graph_snapshot = graph_clone_synth.load();

                    if let Some(ref graph_cell) = **graph_snapshot {
                        // Synthesize samples
                        for sample in buffer.iter_mut() {
                            *sample = graph_cell.0.borrow_mut().process_sample();
                        }

                        // Write to ring buffer
                        let written = ring_producer.push_slice(&buffer);
                        if written < buffer.len() {
                            eprintln!("⚠️  Ring buffer full, dropped {} samples", buffer.len() - written);
                        }
                    } else {
                        // No graph yet, write silence
                        ring_producer.push_slice(&buffer);
                    }
                } else {
                    // Ring buffer is full, sleep briefly
                    thread::sleep(StdDuration::from_micros(100));
                }
            }
        });

        // Audio callback: just reads from ring buffer (FAST!)
        let err_fn = |err| {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/phonon_audio_errors.log")
            {
                let _ = writeln!(file, "[{}] Audio stream error: {}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    err);
            }
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        // Read from ring buffer - MUCH faster than synthesis!
                        let available = ring_consumer.occupied_len();

                        if available >= data.len() {
                            // Ring buffer has enough samples, read them
                            ring_consumer.pop_slice(data);
                        } else {
                            // Underrun: not enough samples in buffer
                            let read = ring_consumer.pop_slice(data);
                            for sample in data[read..].iter_mut() {
                                *sample = 0.0;
                            }

                            static mut UNDERRUN_COUNT: usize = 0;
                            unsafe {
                                UNDERRUN_COUNT += 1;
                                if UNDERRUN_COUNT % 100 == 0 {
                                    eprintln!("⚠️  Audio underrun #{}", UNDERRUN_COUNT);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        let available = ring_consumer.occupied_len();

                        if available >= data.len() {
                            // Read from ring buffer and convert to i16
                            let mut temp = vec![0.0f32; data.len()];
                            ring_consumer.pop_slice(&mut temp);
                            for (dst, src) in data.iter_mut().zip(temp.iter()) {
                                *dst = (*src * 32767.0) as i16;
                            }
                        } else {
                            // Underrun
                            let mut temp = vec![0.0f32; available];
                            ring_consumer.pop_slice(&mut temp);
                            for (i, dst) in data.iter_mut().enumerate() {
                                if i < temp.len() {
                                    *dst = (temp[i] * 32767.0) as i16;
                                } else {
                                    *dst = 0;
                                }
                            }

                            static mut UNDERRUN_COUNT: usize = 0;
                            unsafe {
                                UNDERRUN_COUNT += 1;
                                if UNDERRUN_COUNT % 100 == 0 {
                                    eprintln!("⚠️  Audio underrun #{}", UNDERRUN_COUNT);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => return Err("Unsupported sample format".into()),
        }
        .map_err(|e| format!("Failed to build stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;

        // Load initial content
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

        // Start cursor at beginning of file (not end)
        let cursor_pos = 0;
        let bus_names = completion::extract_bus_names(&content);

        Ok(Self {
            cursor_pos,
            content,
            file_path,
            status_message: "🎵 Ready - C-x: eval | C-u: undo | C-r: redo | Tab: complete | Alt-/: help"
                .to_string(),
            is_playing: false,
            error_message: None,
            graph,
            _stream: stream,
            sample_rate,
            flash_highlight: None,
            kill_buffer: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            console_messages: vec!["Welcome to Phonon Live Coding".to_string()],
            completion_state: completion::CompletionState::new(),
            sample_names: completion::discover_samples(),
            bus_names,
            command_console: CommandConsole::new(),
        })
    }

    /// Load and compile DSL code into the audio graph
    fn load_code(&mut self, code: &str) -> Result<(), String> {
        // Parse the DSL code
        let (rest, statements) = parse_program(code)
            .map_err(|e| format!("Parse error: {}", e))?;

        if !rest.trim().is_empty() {
            return Err(format!("Failed to parse entire code, remaining: {}", rest));
        }

        // Compile into a graph
        // Note: compile_program sets CPS from tempo:/bpm: statements in the code
        // Default is 0.5 CPS if not specified
        let mut new_graph = compile_program(statements, self.sample_rate)
            .map_err(|e| format!("Compile error: {}", e))?;

        // CRITICAL: Preserve cycle position from old graph to prevent timing shift on reload
        // This ensures seamless hot-swapping - the new pattern picks up at the exact
        // same point in the cycle where the old one left off
        let current_graph = self.graph.load();
        if let Some(ref old_graph_cell) = **current_graph {
            let current_cycle = old_graph_cell.0.borrow().get_cycle_position();
            new_graph.set_cycle_position(current_cycle);
        }

        // Hot-swap the graph atomically using lock-free ArcSwap
        // Background synthesis thread will pick up new graph on next render
        self.graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));

        Ok(())
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
        // If command console is visible, route keys to it
        if self.command_console.is_visible() {
            return self.handle_console_key_event(key);
        }

        match key.code {
            // Quit with Alt+Q (Ctrl+Q conflicts with terminal flow control)
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::ALT) => KeyResult::Quit,

            // Alt+/ : Toggle command console
            KeyCode::Char('/') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.command_console.toggle();
                KeyResult::Continue
            }

            // Ctrl+X: Evaluate current block (chunk)
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.eval_chunk();
                KeyResult::Continue
            }

            // Ctrl+U: Undo
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.undo();
                KeyResult::Continue
            }

            // Ctrl+R: Redo
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.redo();
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
                // If completion is visible, navigate through completions
                if self.completion_state.is_visible() {
                    self.cycle_completion_forward();
                } else {
                    self.move_cursor_down(); // Next line
                }
                KeyResult::Continue
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // If completion is visible, navigate through completions
                if self.completion_state.is_visible() {
                    self.cycle_completion_backward();
                } else {
                    self.move_cursor_up(); // Previous line
                }
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
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.yank(); // Yank (paste) from kill buffer
                KeyResult::Continue
            }

            // Tab: trigger completion or accept current selection
            KeyCode::Tab => {
                if self.completion_state.is_visible() {
                    // Second Tab: accept current selection
                    self.accept_completion();
                } else {
                    // First Tab: show completions with first item selected
                    self.trigger_completion();
                }
                KeyResult::Continue
            }
            KeyCode::Esc => {
                // Dismiss completion popup
                self.cancel_completion();
                KeyResult::Continue
            }

            // Regular character input
            KeyCode::Char(c) => {
                self.insert_char(c);
                // Re-filter completions if active
                if self.completion_state.is_visible() {
                    self.update_completion_filter();
                }
                KeyResult::Continue
            }
            KeyCode::Enter => {
                // If completion is active, accept the current suggestion
                if self.completion_state.is_visible() {
                    self.accept_completion();
                } else {
                    self.insert_char('\n');
                }
                KeyResult::Continue
            }
            KeyCode::Backspace => {
                self.delete_char();
                // Re-filter completions if active
                if self.completion_state.is_visible() {
                    self.update_completion_filter();
                }
                KeyResult::Continue
            }
            // Arrow keys still work
            KeyCode::Left => {
                self.cancel_completion();
                self.move_cursor_left();
                KeyResult::Continue
            }
            KeyCode::Right => {
                self.cancel_completion();
                self.move_cursor_right();
                KeyResult::Continue
            }
            KeyCode::Up => {
                // If completion active, cycle up through suggestions
                if self.completion_state.is_visible() {
                    self.cycle_completion_backward();
                } else {
                    self.move_cursor_up();
                }
                KeyResult::Continue
            }
            KeyCode::Down => {
                // If completion active, cycle down through suggestions
                if self.completion_state.is_visible() {
                    self.cycle_completion_forward();
                } else {
                    self.move_cursor_down();
                }
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
        let terminal_width = f.size().width;

        // Smart layout: console on side if width allows (2/3 editor, 1/3 console, min 40 for console)
        // Otherwise console on bottom (portrait mode)
        let use_side_console = terminal_width >= 120; // 80 for editor + 40 for console

        let main_chunks = if use_side_console {
            // Horizontal layout: editor (2/3) | console (1/3)
            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(66), // Editor
                    Constraint::Percentage(34), // Console
                ])
                .split(f.size());

            // Split editor side vertically for status bar
            let editor_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),    // Editor
                    Constraint::Length(3), // Status area
                ])
                .split(horizontal[0]);

            (editor_area[0], editor_area[1], Some(horizontal[1]))
        } else {
            // Vertical layout: editor above console
            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(66), // Editor
                    Constraint::Length(3),      // Status area
                    Constraint::Percentage(34), // Console
                ])
                .split(f.size());

            (vertical[0], vertical[1], Some(vertical[2]))
        };

        let (editor_chunk, status_chunk, console_chunk) = main_chunks;

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

        f.render_widget(paragraph, editor_chunk);

        // Completion popup (if active)
        if self.completion_state.is_visible() {
            let completions = self.completion_state.completions();
            let selected_index = self.completion_state.selected_index();

            let popup_width = 50;
            let popup_y = 3;

            // Calculate max height: from popup_y to bottom of screen
            let max_height = editor_chunk.height.saturating_sub(popup_y + 1);
            let popup_height = (completions.len() + 2).min(max_height as usize) as u16;

            // Position popup near cursor (simplified - center of screen)
            let popup_x = editor_chunk.width.saturating_sub(popup_width + 2).max(2);

            let popup_area = ratatui::layout::Rect {
                x: editor_chunk.x + popup_x,
                y: editor_chunk.y + popup_y,
                width: popup_width,
                height: popup_height,
            };

            // Calculate scroll offset to keep selected item visible
            // popup_height - 2 for borders = visible lines
            let visible_items = (popup_height.saturating_sub(2)) as usize;
            let scroll_offset = if selected_index < visible_items {
                // Near the top, no scrolling needed
                0
            } else {
                // Keep selected item in the middle of visible area when possible
                let ideal_offset = selected_index.saturating_sub(visible_items / 2);
                // But don't scroll past the end
                let max_offset = completions.len().saturating_sub(visible_items);
                ideal_offset.min(max_offset)
            };

            // Build popup content with type labels (only visible items)
            let mut popup_lines = Vec::new();

            // Show scroll indicator at top if there are items above
            if scroll_offset > 0 {
                popup_lines.push(Line::from(Span::styled(
                    "  ▲ more above ▲",
                    Style::default().fg(Color::DarkGray)
                )));
            }

            let visible_completions = completions.iter()
                .skip(scroll_offset)
                .take(visible_items);

            for (displayed_idx, completion) in visible_completions.enumerate() {
                let actual_idx = scroll_offset + displayed_idx;
                let is_selected = actual_idx == selected_index;
                let prefix = if is_selected { "► " } else { "  " };
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };

                // Format: "  completion_text     [type]"
                let line_text = format!("{}{:20} {}",
                    prefix,
                    completion.text,
                    completion.label()
                );

                popup_lines.push(Line::from(Span::styled(line_text, style)));
            }

            // Show scroll indicator at bottom if there are items below
            if scroll_offset + visible_items < completions.len() {
                popup_lines.push(Line::from(Span::styled(
                    "  ▼ more below ▼",
                    Style::default().fg(Color::DarkGray)
                )));
            }

            let popup_block = Block::default()
                .title("Completions")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan).bg(Color::Black));

            let popup_paragraph = Paragraph::new(popup_lines)
                .block(popup_block)
                .style(Style::default().bg(Color::Black));

            f.render_widget(popup_paragraph, popup_area);
        }

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

        let help_text = "C-x: Eval | C-u: Undo | C-r: Redo | C-k: Kill | C-y: Yank | C-h: Hush | C-s: Save | Alt-q: Quit";

        let status_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Status message
                Constraint::Length(1), // Help text
            ])
            .split(status_chunk);

        let status_paragraph = Paragraph::new(status_text)
            .style(status_style)
            .alignment(Alignment::Left);

        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(status_paragraph, status_chunks[0]);
        f.render_widget(help_paragraph, status_chunks[1]);

        // Console area
        if let Some(console_area) = console_chunk {
            let console_title = format!("Console ({})", self.console_messages.len());
            let console_block = Block::default()
                .title(console_title)
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan));

            // Show last N messages that fit in the console area
            let console_height = console_area.height.saturating_sub(2) as usize; // -2 for borders
            let start_idx = self.console_messages.len().saturating_sub(console_height);
            let visible_messages: Vec<Line> = self.console_messages[start_idx..]
                .iter()
                .map(|msg| Line::from(msg.as_str()))
                .collect();

            let console_paragraph = Paragraph::new(visible_messages)
                .block(console_block)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::White));

            f.render_widget(console_paragraph, console_area);
        }

        // Command console overlay (rendered on top of everything)
        if self.command_console.is_visible() {
            // Create centered popup area (80% width, 60% height)
            let area = f.size();
            let popup_width = (area.width as f32 * 0.8) as u16;
            let popup_height = (area.height as f32 * 0.6) as u16;
            let popup_x = (area.width - popup_width) / 2;
            let popup_y = (area.height - popup_height) / 2;

            let popup_area = ratatui::layout::Rect {
                x: popup_x,
                y: popup_y,
                width: popup_width,
                height: popup_height,
            };

            self.command_console.render(f, popup_area);
        }
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
        let (flash_start, flash_end, flash_color) =
            if let Some((start, end, frames)) = self.flash_highlight {
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
                        spans.push(Span::styled(
                            "█",
                            Style::default().fg(Color::White).bg(flash_color),
                        ));
                    } else {
                        spans.push(Span::styled("█", Style::default().fg(Color::White)));
                    }
                } else if cursor_col < line_text.len() {
                    // Cursor in middle of line - highlight whole line, then add cursor
                    let mut highlighted = highlight_line(line_text);

                    // Find which character position cursor is at
                    let mut char_count = 0;
                    let mut modified_spans = Vec::new();

                    for mut span in highlighted {
                        let span_len = span.content.chars().count();

                        if char_count + span_len <= cursor_col {
                            // Cursor is after this span entirely
                            if is_flashing {
                                span.style = span.style.bg(flash_color).fg(Color::Black);
                            }
                            modified_spans.push(span);
                            char_count += span_len;
                        } else if char_count > cursor_col {
                            // Cursor is before this span entirely
                            if is_flashing {
                                span.style = span.style.bg(flash_color).fg(Color::Black);
                            }
                            modified_spans.push(span);
                            char_count += span_len;
                        } else {
                            // Cursor is WITHIN this span
                            let cursor_offset = cursor_col - char_count;
                            let chars: Vec<char> = span.content.chars().collect();

                            // Before cursor in this span
                            if cursor_offset > 0 {
                                let before: String = chars[..cursor_offset].iter().collect();
                                let mut before_style = span.style;
                                if is_flashing {
                                    before_style = before_style.bg(flash_color).fg(Color::Black);
                                }
                                modified_spans.push(Span::styled(before, before_style));
                            }

                            // Cursor character - white background
                            let cursor_char = chars[cursor_offset].to_string();
                            modified_spans.push(Span::styled(
                                cursor_char,
                                Style::default().bg(Color::White).fg(Color::Black),
                            ));

                            // After cursor in this span
                            if cursor_offset + 1 < chars.len() {
                                let after: String = chars[cursor_offset + 1..].iter().collect();
                                let mut after_style = span.style;
                                if is_flashing {
                                    after_style = after_style.bg(flash_color).fg(Color::Black);
                                }
                                modified_spans.push(Span::styled(after, after_style));
                            }

                            char_count += span_len;
                        }
                    }

                    spans = modified_spans;
                } else {
                    // Cursor at end of line
                    let mut highlighted = highlight_line(line_text);
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
                        lines.push(Line::from(Span::styled(
                            " ",
                            Style::default().bg(flash_color),
                        )));
                    } else {
                        lines.push(Line::from(Span::raw(" "))); // Ensure empty lines take space
                    }
                } else {
                    let mut spans = highlight_line(line_text);
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
        // Save state for undo (batch consecutive chars for efficiency)
        if c == '\n' || self.undo_stack.is_empty() {
            self.push_undo();
        }
        self.content.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.error_message = None;
    }

    /// Delete character before cursor
    fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.push_undo();
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
            self.push_undo();
            self.content.remove(self.cursor_pos);
        }
        self.error_message = None;
    }

    /// Kill to end of line (Ctrl+K) - saves to kill buffer
    fn kill_line(&mut self) {
        self.push_undo();
        let lines: Vec<&str> = self.content.split('\n').collect();
        let mut current_pos = 0;

        for line in lines.iter() {
            if current_pos + line.len() >= self.cursor_pos {
                // Found current line
                let line_start = current_pos;
                let line_end = current_pos + line.len();

                if self.cursor_pos < line_end {
                    // Save killed text to kill buffer
                    self.kill_buffer = self.content[self.cursor_pos..line_end].to_string();
                    // Remove from cursor to end of line
                    self.content.drain(self.cursor_pos..line_end);
                } else {
                    // At end of line - kill the newline if it exists
                    if self.cursor_pos < self.content.len() {
                        self.kill_buffer = "\n".to_string();
                        self.content.remove(self.cursor_pos);
                    }
                }
                break;
            }
            current_pos += line.len() + 1; // +1 for newline
        }
        self.error_message = None;
    }

    /// Yank (paste) from kill buffer (Ctrl+Y)
    fn yank(&mut self) {
        if !self.kill_buffer.is_empty() {
            self.push_undo();
            self.content.insert_str(self.cursor_pos, &self.kill_buffer);
            self.cursor_pos += self.kill_buffer.len();
        }
        self.error_message = None;
    }

    /// Push current state to undo stack
    fn push_undo(&mut self) {
        // Limit undo stack size to 100 states
        if self.undo_stack.len() >= 100 {
            self.undo_stack.remove(0);
        }
        self.undo_stack
            .push((self.content.clone(), self.cursor_pos));
        // Clear redo stack on new edit
        self.redo_stack.clear();
    }

    /// Undo last change (Ctrl+U)
    fn undo(&mut self) {
        if let Some((content, cursor_pos)) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack
                .push((self.content.clone(), self.cursor_pos));
            // Restore previous state
            self.content = content;
            self.cursor_pos = cursor_pos;
            self.status_message = "↶ Undo".to_string();
            self.add_console_message("Undo");
        } else {
            self.status_message = "⚠️  Nothing to undo".to_string();
        }
        self.error_message = None;
    }

    /// Redo last undone change (Ctrl+R)
    fn redo(&mut self) {
        if let Some((content, cursor_pos)) = self.redo_stack.pop() {
            // Save current state to undo stack
            self.undo_stack
                .push((self.content.clone(), self.cursor_pos));
            // Restore next state
            self.content = content;
            self.cursor_pos = cursor_pos;
            self.status_message = "↷ Redo".to_string();
            self.add_console_message("Redo");
        } else {
            self.status_message = "⚠️  Nothing to redo".to_string();
        }
        self.error_message = None;
    }

    /// Add message to console
    fn add_console_message(&mut self, msg: &str) {
        self.console_messages.push(msg.to_string());
        // Keep last 50 messages
        if self.console_messages.len() > 50 {
            self.console_messages.remove(0);
        }
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
        self.error_message = None;
        self.status_message = "🔄 Reloading pattern...".to_string();

        // Clone content to avoid borrow checker issues
        let content = self.content.clone();

        // Load the code into the graph
        if let Err(e) = self.load_code(&content) {
            self.error_message = Some(format!("Failed to load: {e}"));
        } else {
            self.status_message = "✅ Pattern reloaded!".to_string();
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

        self.error_message = None;
        self.status_message = format!("🔄 Evaluating chunk ({} chars)...", chunk.len());

        // Collect data before evaluation
        let preview = chunk.lines().take(2).collect::<Vec<_>>().join(" ");
        let preview_short = if preview.len() > 60 {
            format!("{}...", &preview[..60])
        } else {
            preview
        };
        let bus_count = self
            .content
            .lines()
            .filter(|l| l.trim().starts_with("~"))
            .count();
        let has_out = self
            .content
            .lines()
            .any(|l| l.trim().starts_with("out:") || l.trim().starts_with("out "));

        // Check if chunk starts with "hush" - if so, clear audio first
        let did_hush = if chunk.trim_start().starts_with("hush") {
            self.hush();
            true
        } else {
            false
        };

        // Clone content to avoid borrow checker issues
        let content = self.content.clone();

        // IMPORTANT: Send the full session content, not just the chunk!
        // This ensures all buses, tempo, and output assignments are preserved.
        let result = self.load_code(&content);

        // Now we can mutate self safely - add all console messages
        self.add_console_message(&format!("📝 Evaluating: {} chars", chunk.len()));
        self.add_console_message(&format!("   {}", preview_short));

        if did_hush {
            self.add_console_message("🔇 Hush - clearing audio");
        }

        if let Err(e) = result {
            self.error_message = Some(format!("Eval failed: {e}"));
            self.add_console_message(&format!("❌ Parse error: {e}"));
        } else {
            self.status_message = "✅ Chunk evaluated!".to_string();
            self.add_console_message("✅ Sent to engine");
            self.add_console_message(&format!(
                "   {} buses, out: {}",
                bus_count,
                if has_out { "yes" } else { "NO!" }
            ));

            // Flash the evaluated chunk: 10 frames = 500ms (pop + fade)
            self.flash_highlight = Some((start_line, end_line, 10));
        }
    }

    /// Evaluate entire session - Ctrl-R (Reload)
    fn eval_all(&mut self) {
        if self.content.trim().is_empty() {
            self.status_message = "⚠️  Empty session".to_string();
            return;
        }

        self.error_message = None;
        self.status_message = "🔄 Reloading entire session...".to_string();

        // Clone content to avoid borrow checker issues
        let content = self.content.clone();

        if let Err(e) = self.load_code(&content) {
            self.error_message = Some(format!("Reload failed: {e}"));
        } else {
            self.status_message = "✅ Session reloaded!".to_string();
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
        // Clear the graph to silence all sound
        self.graph.store(Arc::new(None));
        self.status_message = "🔇 Hushed - C-r to reload".to_string();
    }

    /// Panic - stop everything
    fn panic(&mut self) {
        // Clear the graph to stop everything
        self.graph.store(Arc::new(None));
        self.status_message = "🚨 PANIC! All stopped - C-r to restart".to_string();
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

    // ==================== TAB COMPLETION ====================

    /// Get all available completion candidates
    fn get_all_completions(&self) -> Vec<String> {
        let mut completions = Vec::new();

        // Built-in functions (from syntax highlighter)
        let functions = vec![
            "s",
            "euclid",
            "fast",
            "slow",
            "rev",
            "every",
            "degrade",
            "degradeBy",
            "stutter",
            "palindrome",
            "sine",
            "saw",
            "square",
            "tri",
            "lpf",
            "hpf",
            "bpf",
            "notch",
            "reverb",
            "delay",
            "chorus",
            "bitcrush",
            "distortion",
            "tempo",
            "out",
            "out1",
            "out2",
            "out3",
            "out4",
            "out5",
            "out6",
            "out7",
            "out8",
            "hush",
            "panic",
        ];
        completions.extend(functions.iter().map(|s| s.to_string()));

        // Extract bus names from content (lines starting with ~name:)
        for line in self.content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix('~') {
                // Extract the bus name (everything before : or space)
                if let Some(colon_pos) = rest.find(':') {
                    let bus_name = rest[..colon_pos].trim();
                    if !bus_name.is_empty() {
                        completions.push(format!("~{}", bus_name));
                    }
                }
            }
        }

        // Remove duplicates and sort
        completions.sort();
        completions.dedup();
        completions
    }

    /// Get the word at cursor and its start position
    fn get_word_at_cursor(&self) -> (String, usize) {
        if self.cursor_pos == 0 {
            return (String::new(), 0);
        }

        // Find the start of the current word (alphanumeric, _, ~)
        let mut word_start = self.cursor_pos;
        let chars: Vec<char> = self.content.chars().collect();

        while word_start > 0 {
            let prev_char = chars[word_start - 1];
            if prev_char.is_alphanumeric() || prev_char == '_' || prev_char == '~' {
                word_start -= 1;
            } else {
                break;
            }
        }

        // Extract the word
        let word: String = chars[word_start..self.cursor_pos].iter().collect();
        (word, word_start)
    }

    /// Convert absolute cursor position to (line_index, column)
    fn pos_to_line_col(&self, pos: usize) -> (usize, usize) {
        let mut current_pos = 0;
        for (line_idx, line) in self.content.split('\n').enumerate() {
            let line_len = line.len();
            if current_pos + line_len >= pos {
                return (line_idx, pos - current_pos);
            }
            current_pos += line_len + 1; // +1 for newline
        }
        // If we're at the end, return last line
        let line_count = self.content.split('\n').count();
        (line_count.saturating_sub(1), 0)
    }

    /// Trigger completion or cycle to next if already active
    fn trigger_or_cycle_completion(&mut self) {
        if self.completion_state.is_visible() {
            // Already completing - cycle forward
            self.completion_state.next();
        } else {
            self.trigger_completion();
        }
    }

    /// Trigger completion popup (first Tab press)
    fn trigger_completion(&mut self) {
        // Update bus names first
        self.bus_names = completion::extract_bus_names(&self.content);

        // Get current line
        let lines: Vec<&str> = self.content.split('\n').collect();
        let (line_idx, col) = self.pos_to_line_col(self.cursor_pos);

        if line_idx >= lines.len() {
            return;
        }

        let line = lines[line_idx];

        // Get context first to determine if we should complete
        let context = completion::get_completion_context(line, col);

        // Get token at cursor (may be None if cursor is at empty space)
        let token = completion::get_token_at_cursor(line, col);

        // Determine partial text and token start position
        let (partial_text, token_start) = match token {
            Some(t) => (t.text.clone(), t.start),
            None => {
                // No token found - check if we're in a context that allows empty completion
                match context {
                    completion::CompletionContext::Sample | completion::CompletionContext::Bus => {
                        // Inside string - show all completions
                        ("".to_string(), col)
                    }
                    _ => {
                        // Not in a completable context
                        return;
                    }
                }
            }
        };

        // Get completions
        let completions = completion::filter_completions(
            &partial_text,
            &context,
            &self.sample_names,
            &self.bus_names,
        );

        if completions.is_empty() {
            self.status_message = "No completions found".to_string();
            // Flash visual feedback for no matches
            self.flash_highlight = Some((line_idx, line_idx, 3));
            return;
        }

        // Show completions
        let line_start = self.content[..self.cursor_pos]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);

        self.completion_state.show(
            completions.clone(),
            partial_text,
            line_start + token_start,
        );

        self.status_message = format!(
            "{} completions | Tab/↑↓: navigate | Enter: accept | Esc: cancel",
            completions.len()
        );
    }

    /// Cycle completion forward (Down arrow)
    fn cycle_completion_forward(&mut self) {
        self.completion_state.next();
    }

    /// Cycle completion backward (Up arrow)
    fn cycle_completion_backward(&mut self) {
        self.completion_state.previous();
    }

    /// Accept the current completion (Enter)
    fn accept_completion(&mut self) {
        if let Some(completion) = self.completion_state.accept() {
            // Replace the token at cursor with the completion
            let token_start = self.completion_state.token_start();
            let token_end = self.cursor_pos;

            if token_start < self.content.len() {
                self.content
                    .replace_range(token_start..token_end, &completion.text);
                self.cursor_pos = token_start + completion.text.len();
            }

            self.status_message = format!("✓ {}", completion.text);
        }
    }

    /// Cancel completion (Esc or movement)
    fn cancel_completion(&mut self) {
        self.completion_state.hide();
    }

    /// Update completion filter based on current cursor position
    /// Used when typing to narrow down completions in real-time
    fn update_completion_filter(&mut self) {
        // Update bus names
        self.bus_names = completion::extract_bus_names(&self.content);

        // Get current line and column
        let lines: Vec<&str> = self.content.split('\n').collect();
        let (line_idx, col) = self.pos_to_line_col(self.cursor_pos);

        if line_idx >= lines.len() {
            self.cancel_completion();
            return;
        }

        let line = lines[line_idx];

        // Get context
        let context = completion::get_completion_context(line, col);

        // Get token (may be None if at empty space)
        let token = completion::get_token_at_cursor(line, col);

        // Determine partial text
        let (partial_text, token_start) = match token {
            Some(t) => (t.text.clone(), t.start),
            None => {
                // No token - check if we're in a completable context
                match context {
                    completion::CompletionContext::Sample | completion::CompletionContext::Bus => {
                        ("".to_string(), col)
                    }
                    _ => {
                        self.cancel_completion();
                        return;
                    }
                }
            }
        };

        // Filter completions
        let completions = completion::filter_completions(
            &partial_text,
            &context,
            &self.sample_names,
            &self.bus_names,
        );

        if completions.is_empty() {
            // No matches - hide completion
            self.cancel_completion();
            return;
        }

        // Update completion state with new filtered results
        let line_start = self.content[..self.cursor_pos]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);

        self.completion_state.show(
            completions.clone(),
            partial_text,
            line_start + token_start,
        );

        self.status_message = format!(
            "{} completions | Tab/↑↓: navigate | Enter: accept | Esc: cancel",
            completions.len()
        );
    }

    /// Handle key events when command console is visible
    fn handle_console_key_event(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            // Esc or Alt+/ : Close console
            KeyCode::Esc => {
                self.command_console.hide();
                KeyResult::Continue
            }
            KeyCode::Char('/') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.command_console.toggle();
                KeyResult::Continue
            }

            // Enter : Execute command
            KeyCode::Enter => {
                self.command_console.execute_command();
                KeyResult::Continue
            }

            // Character input
            KeyCode::Char(c) => {
                self.command_console.insert_char(c);
                KeyResult::Continue
            }

            // Backspace
            KeyCode::Backspace => {
                self.command_console.delete_char();
                KeyResult::Continue
            }

            // Arrow keys for cursor movement
            KeyCode::Left => {
                self.command_console.cursor_left();
                KeyResult::Continue
            }
            KeyCode::Right => {
                self.command_console.cursor_right();
                KeyResult::Continue
            }

            _ => KeyResult::Continue,
        }
    }
}

/// Key event result
enum KeyResult {
    Continue,
    Quit,
    Play,
    Save,
}
