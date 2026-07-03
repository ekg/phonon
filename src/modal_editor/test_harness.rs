//! Test harness for modal editor integration tests
//!
//! Provides a testable interface to the editor without requiring
//! terminal UI or audio output.

use super::*;
use crate::unified_graph::LiveClock;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::cell::RefMut;

/// Test harness for scripting editor interactions
pub struct EditorTestHarness {
    editor: ModalEditor,
    /// Persistent sample-advancing live clock, shared across `render_live_chunks`
    /// calls so a graph swap between calls continues from the current position —
    /// mirrors the single clock held by the real synth thread (mod.rs).
    live_clock: Option<LiveClock>,
    /// Heap address of the render-owned graph last rendered, for the ABA-safe
    /// "did the graph swap?" check across `render_live_chunks` calls. Replaces the
    /// old `Arc<Option<GraphCell>>` identity now that the graph is single-owner.
    prev_graph_addr: Option<usize>,
}

impl EditorTestHarness {
    /// Drive the headless render side once (pull the first graph / apply pending
    /// swap / hush / panic commands, drop retired graphs) and return a handle to
    /// the local render owner. In headless mode the harness *is* the render owner,
    /// so this is where a queued swap actually lands on the graph. Returns `None`
    /// only if the editor was not built headless (no local render side).
    fn synced(&self) -> Option<RefMut<'_, LocalRender>> {
        let rl_cell = self.editor.render_local.as_ref()?;
        let mut rl = rl_cell.borrow_mut();
        rl.sync();
        Some(rl)
    }
}

impl EditorTestHarness {
    /// Create a new test harness with an empty editor
    /// Uses headless mode to work in CI environments without audio hardware
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut editor = ModalEditor::new_headless()?;
        // Clear the default template content for clean testing
        editor.content = String::new();
        editor.cursor_pos = 0;
        Ok(Self {
            editor,
            live_clock: None,
            prev_graph_addr: None,
        })
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
            let event = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
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
        self.editor
            .completion_state
            .completions()
            .iter()
            .map(|c| c.text.clone())
            .collect()
    }

    /// Get the selected completion (if any)
    pub fn selected_completion(&self) -> Option<String> {
        self.editor
            .completion_state
            .selected_completion()
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
            option,
            options
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
            selected,
            Some(expected.to_string()),
            "\nExpected selected: {:?}\nActual selected: {:?}",
            Some(expected),
            selected
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
        let rl = self.synced()?;
        rl.cur.as_ref().map(|g| g.get_cps())
    }

    /// Get cycle position from the current graph (if loaded)
    pub fn get_cycle_position(&self) -> Option<f64> {
        let rl = self.synced()?;
        rl.cur.as_ref().map(|g| g.get_cycle_position())
    }

    /// Check if wall-clock timing is enabled
    pub fn is_wall_clock_enabled(&self) -> Option<bool> {
        let rl = self.synced()?;
        rl.cur.as_ref().map(|g| g.use_wall_clock)
    }

    /// Check if a graph is loaded
    pub fn has_graph(&self) -> bool {
        self.synced().map(|rl| rl.cur.is_some()).unwrap_or(false)
    }

    /// Set content directly (for test setup)
    pub fn set_content(&mut self, content: &str) -> &mut Self {
        self.editor.content = content.to_string();
        self.editor.cursor_pos = content.len();
        self
    }

    /// Process audio chunks through the graph (simulating real-time audio callback)
    /// Returns the number of chunks processed, or panics on timeout
    /// This tests the exact code path used by phonon edit's audio thread
    pub fn process_audio_chunks(
        &self,
        num_chunks: usize,
        timeout_ms: u64,
    ) -> Result<usize, String> {
        use std::time::{Duration, Instant};

        let mut rl = self.synced().ok_or("No render side (not headless)")?;
        let graph = match rl.cur.as_mut() {
            Some(g) => g,
            None => return Err("No graph loaded".to_string()),
        };
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let mut buffer = [0.0f32; 512];
        let frames = buffer.len() / 2; // stereo-interleaved frames of cycle-time
        let mut processed = 0;
        // Sample-advancing live clock — the migrated synth code path
        // (src/modal_editor/mod.rs). Advancing by samples emitted (NOT the wall
        // clock) keeps the pattern on the sample grid even when rendered ahead
        // of real time, fixing pt-F1 onset clustering.
        let mut clock: Option<LiveClock> = None;

        for i in 0..num_chunks {
            if start.elapsed() > timeout {
                return Err(format!(
                    "Timeout after {} chunks ({}ms)! Possible infinite loop.",
                    i, timeout_ms
                ));
            }

            let chunk_start = Instant::now();

            if clock.is_none() {
                // Seed from the graph's compiled start position (honours
                // setCycle/resetCycles), like the synth loop's first graph.
                clock = Some(LiveClock::new(
                    graph.sample_rate(),
                    graph.get_cps(),
                    graph.get_cycle_position(),
                ));
            }
            let c = clock.as_mut().unwrap();
            c.set_cps(graph.get_cps());
            let (start_cycle, increment, cps) = c.advance_buffer(frames);
            graph.process_buffer_at(&mut buffer, start_cycle, increment, cps);
            processed += 1;

            let chunk_time = chunk_start.elapsed();
            // If any chunk takes > 100ms, something is very wrong
            if chunk_time.as_millis() > 100 {
                return Err(format!(
                    "Chunk {} took {:?} - exceeds real-time budget!",
                    i, chunk_time
                ));
            }
        }

        Ok(processed)
    }

    /// Enable wall-clock timing on the graph (mimics modal editor behavior)
    pub fn enable_wall_clock_timing(&self) -> Result<(), String> {
        let mut rl = self.synced().ok_or("No render side (not headless)")?;
        match rl.cur.as_mut() {
            Some(graph) => {
                graph.enable_wall_clock_timing();
                Ok(())
            }
            None => Err("No graph loaded".to_string()),
        }
    }

    /// Render `num_chunks` mono chunks through the migrated synth code path
    /// (LiveClock + `process_buffer_at`), as fast as possible (ring-fill), and
    /// return the mono (left-channel) audio. This is the audio-capturing twin of
    /// [`process_audio_chunks`](Self::process_audio_chunks) and exercises the exact
    /// timing the real synth thread uses, so onsets land on the sample grid (pt-F1).
    pub fn process_audio_chunks_capture(&self, num_chunks: usize) -> Result<Vec<f32>, String> {
        let mut rl = self.synced().ok_or("No render side (not headless)")?;
        let graph = match rl.cur.as_mut() {
            Some(g) => g,
            None => return Err("No graph loaded".to_string()),
        };

        let mut buffer = [0.0f32; 512];
        let frames = buffer.len() / 2;
        // Seed from the graph's compiled start position (honours setCycle), like
        // the synth loop's first graph.
        let mut clock =
            LiveClock::new(graph.sample_rate(), graph.get_cps(), graph.get_cycle_position());
        let mut out = Vec::with_capacity(num_chunks * frames);
        for _ in 0..num_chunks {
            buffer.iter_mut().for_each(|s| *s = 0.0);
            clock.set_cps(graph.get_cps());
            let (start_cycle, increment, cps) = clock.advance_buffer(frames);
            graph.process_buffer_at(&mut buffer, start_cycle, increment, cps);
            for i in 0..frames {
                out.push(buffer[i * 2]);
            }
        }
        Ok(out)
    }

    /// Render `num_chunks` mono chunks the way the OLD (buggy) synth thread did:
    /// `process_buffer` in wall-clock mode with the wall clock frozen between
    /// renders (the deterministic worst case of the synth thread rendering far
    /// ahead of real time while filling the ring). Every chunk re-anchors
    /// `buffer_start_cycle` to the same band, so the pattern stalls and onsets
    /// cluster (pt-F1). Used to pin the bug the LiveClock migration removes.
    pub fn render_ring_fill_wall_clock_frozen(
        &self,
        num_chunks: usize,
    ) -> Result<Vec<f32>, String> {
        let mut rl = self.synced().ok_or("No render side (not headless)")?;
        let graph = match rl.cur.as_mut() {
            Some(g) => g,
            None => return Err("No graph loaded".to_string()),
        };
        graph.enable_wall_clock_timing();

        let mut buffer = [0.0f32; 512];
        let frames = buffer.len() / 2;
        let mut out = Vec::with_capacity(num_chunks * frames);
        for _ in 0..num_chunks {
            buffer.iter_mut().for_each(|s| *s = 0.0);
            // Freeze the wall clock: reset the session origin so elapsed ≈ 0 each
            // render (mirrors tests/live_clock_timing.rs::render_via_wall_clock_frozen).
            graph.session_start_time = std::time::Instant::now();
            graph.process_buffer(&mut buffer);
            for i in 0..frames {
                out.push(buffer[i * 2]);
            }
        }
        Ok(out)
    }

    /// Render `num_chunks` mono chunks through the synth code path using the
    /// harness's **persistent** live clock (held across calls). A graph swap
    /// between two calls (e.g. a Ctrl+X re-eval) is detected by the owned graph's
    /// heap address and the swapped-in graph is seeded from the clock's current
    /// position — exactly the reload-continuity path of the real synth thread
    /// (mod.rs). Returns the mono (left-channel) audio.
    pub fn render_live_chunks(&mut self, num_chunks: usize) -> Result<Vec<f32>, String> {
        // Drive the local render owner (pulls the first graph / applies a pending
        // swap into `cur`), then render the now-current graph.
        let rl_cell = self
            .editor
            .render_local
            .as_ref()
            .ok_or("No render side (not headless)")?;
        let mut rl = rl_cell.borrow_mut();
        rl.sync();
        let graph = match rl.cur.as_mut() {
            Some(g) => g,
            None => return Err("No graph loaded".to_string()),
        };

        // ABA-safe swap detection: the render owner replaces the owned box on a
        // swap, so a changed heap address means "the graph was swapped".
        let cur_addr = &**graph as *const UnifiedSignalGraph as usize;
        let is_new_graph = self.prev_graph_addr != Some(cur_addr);

        // Seed or rebase the clock for this graph once (matches the synth loop's
        // per-swap handling: first graph seeds the clock; a swapped-in graph is
        // seeded from the clock so it continues from the current position with no
        // re-trigger burst, rebasing tempo without teleporting — pt-F2).
        match self.live_clock.as_mut() {
            None => {
                self.live_clock = Some(LiveClock::new(
                    graph.sample_rate(),
                    graph.get_cps(),
                    graph.get_cycle_position(),
                ));
            }
            Some(c) => {
                c.set_cps(graph.get_cps());
                if is_new_graph {
                    graph.set_cycle_position(c.position());
                }
            }
        }

        let mut buffer = [0.0f32; 512];
        let frames = buffer.len() / 2;
        let mut out = Vec::with_capacity(num_chunks * frames);
        for _ in 0..num_chunks {
            buffer.iter_mut().for_each(|s| *s = 0.0);
            let c = self.live_clock.as_mut().unwrap();
            c.set_cps(graph.get_cps());
            let (start_cycle, increment, cps) = c.advance_buffer(frames);
            graph.process_buffer_at(&mut buffer, start_cycle, increment, cps);
            for i in 0..frames {
                out.push(buffer[i * 2]);
            }
        }
        self.prev_graph_addr = Some(cur_addr);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_basic_typing() {
        let mut harness = EditorTestHarness::new().unwrap();
        harness.type_text("hello").assert_line("hello");
    }

    /// Simulate 30 seconds of realtime audio with d.ph to measure underruns
    /// Note: This test is ignored by default because it depends on a d.ph file
    /// that must be manually created. Run with --ignored to include it.
    #[test]
    #[ignore = "HARDWARE: requires d.ph file"]
    fn test_d_ph_realtime_simulation() {
        use std::time::{Duration, Instant};
        use crate::compositional_compiler::compile_program;
        use crate::compositional_parser::parse_program;
        use std::cell::RefCell;

        // Load d.ph content
        let d_ph_content = std::fs::read_to_string("d.ph").expect("Failed to read d.ph");

        // Parse and compile directly (like modal editor does)
        let (_, statements) = parse_program(&d_ph_content).expect("Failed to parse d.ph");
        let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile d.ph");

        // Enable wall-clock timing (like modal editor does)
        graph.enable_wall_clock_timing();

        // Preload samples (like modal editor does)
        graph.preload_samples();

        // Wrap in RefCell to simulate modal editor's GraphCell
        let graph_cell = RefCell::new(graph);

        // Simulate 30 seconds of realtime audio
        // At 44100 Hz with 512-sample buffers, that's ~86 buffers/second
        // For 30 seconds: 30 * 86 = 2580 buffers
        let duration_secs = 30.0;
        let buffers_per_second = 44100.0 / 512.0; // ~86.13
        let total_buffers = (duration_secs * buffers_per_second) as usize;
        let buffer_duration = Duration::from_secs_f64(512.0 / 44100.0); // ~11.6ms

        println!("\n=== d.ph Realtime Simulation ({}s) ===", duration_secs);
        println!("Total buffers to process: {}", total_buffers);
        println!("Budget per buffer: {:?}", buffer_duration);

        let start = Instant::now();
        let mut buffer = [0.0f32; 512];
        let mut processed = 0;
        let mut underruns = 0;
        let mut max_time_us = 0u128;
        let mut total_time_us = 0u128;
        let mut times_us: Vec<u128> = Vec::with_capacity(total_buffers);

        // Process buffers at realtime pace
        for i in 0..total_buffers {
            let expected_time = Duration::from_secs_f64(i as f64 / buffers_per_second);

            // Wait until we're at the right time (simulating realtime)
            while start.elapsed() < expected_time {
                std::thread::sleep(Duration::from_micros(100));
            }

            let chunk_start = Instant::now();

            match graph_cell.try_borrow_mut() {
                Ok(mut graph) => {
                    graph.process_buffer(&mut buffer);
                    processed += 1;
                }
                Err(_) => {
                    underruns += 1;
                    continue;
                }
            }

            let chunk_time = chunk_start.elapsed();
            let chunk_us = chunk_time.as_micros();
            times_us.push(chunk_us);
            total_time_us += chunk_us;

            if chunk_us > max_time_us {
                max_time_us = chunk_us;
            }

            // Check if we exceeded budget (underrun)
            if chunk_time > buffer_duration {
                underruns += 1;
                if underruns <= 10 {
                    let voice_count = graph_cell
                        .try_borrow()
                        .map(|g| g.active_voice_count())
                        .unwrap_or(0);
                    println!(
                        "  ⚠️  Underrun #{}: buffer {} took {:?} ({}% budget) | voices: {}",
                        underruns,
                        i,
                        chunk_time,
                        chunk_us * 100 / 11610,
                        voice_count
                    );
                }
            }

            // Progress update every 5 seconds
            if i > 0 && i % (5 * 86) == 0 {
                println!(
                    "  Progress: {:.0}s - {} underruns so far",
                    i as f64 / buffers_per_second,
                    underruns
                );
            }
        }

        let total_elapsed = start.elapsed();

        // Calculate statistics
        times_us.sort();
        let min_us = times_us.first().copied().unwrap_or(0);
        let median_us = times_us.get(times_us.len() / 2).copied().unwrap_or(0);
        let p95_us = times_us
            .get((times_us.len() as f64 * 0.95) as usize)
            .copied()
            .unwrap_or(0);
        let avg_us = if processed > 0 {
            total_time_us / processed as u128
        } else {
            0
        };

        println!("\n=== Results ===");
        println!("Duration: {:?}", total_elapsed);
        println!("Buffers processed: {}/{}", processed, total_buffers);
        println!("Underruns: {} ({:.1}%)", underruns, underruns as f64 * 100.0 / total_buffers as f64);
        println!("\nTiming (budget: 11610 µs):");
        println!("  Min:    {} µs ({:.1}%)", min_us, min_us as f64 * 100.0 / 11610.0);
        println!("  Avg:    {} µs ({:.1}%)", avg_us, avg_us as f64 * 100.0 / 11610.0);
        println!("  Median: {} µs ({:.1}%)", median_us, median_us as f64 * 100.0 / 11610.0);
        println!("  P95:    {} µs ({:.1}%)", p95_us, p95_us as f64 * 100.0 / 11610.0);
        println!("  Max:    {} µs ({:.1}%)", max_time_us, max_time_us as f64 * 100.0 / 11610.0);

        if underruns > 0 {
            println!("\n❌ FAILED: {} underruns detected!", underruns);
        } else {
            println!("\n✅ PASSED: No underruns!");
        }
    }

    #[test]
    fn test_harness_multiline() {
        let mut harness = EditorTestHarness::new().unwrap();
        harness
            .type_text("line1")
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
