#![allow(unused_assignments, unused_mut)]
#![allow(dead_code)]
//! Modal live coding editor with terminal UI
//!
//! Provides a full-screen text editor for writing Phonon DSL code with
//! real-time audio generation using ring buffer architecture for parallel synthesis

mod command_console;
pub mod completion;
mod highlighting;
mod plugin_browser;
pub mod test_harness;

use command_console::CommandConsole;
use highlighting::highlight_line;
use plugin_browser::PluginBrowser;

use crate::compositional_compiler::compile_program;
use crate::compositional_parser::parse_program;
use crate::midi_input::{MidiEvent, MidiInputHandler, MidiMessageType, MidiRecorder};
use crate::plugin_host::PluginInstanceManager;
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
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration as StdDuration;

// VST3 GUI support (Linux only, with vst3 feature)
#[cfg(all(target_os = "linux", feature = "vst3"))]
use rack::Vst3Gui;

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
    /// Audio stream (kept alive) - None in headless mode for testing
    _stream: Option<cpal::Stream>,
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
    /// Underrun counter (shared with audio callback)
    underrun_count: Arc<AtomicUsize>,
    /// Synthesis performance stats (shared with synthesis thread)
    synth_time_us: Arc<AtomicUsize>,
    /// Ring buffer fill level (0-100%)
    ring_fill_percent: Arc<AtomicUsize>,
    /// Signal to clear ring buffer on next audio callback (instant transitions)
    should_clear_ring: Arc<AtomicBool>,
    /// MIDI input handler
    midi_input: Option<MidiInputHandler>,
    /// MIDI recorder for capturing patterns
    midi_recorder: Option<MidiRecorder>,
    /// Whether MIDI recording is active
    midi_recording: bool,
    /// Recorded MIDI pattern (ready to insert)
    midi_recorded_pattern: Option<String>,
    /// Recorded MIDI pattern as n-offsets (ready to insert)
    midi_recorded_n_pattern: Option<String>,
    /// Recorded MIDI velocity pattern (ready to insert)
    midi_recorded_velocity: Option<String>,
    /// Recorded MIDI legato pattern (ready to insert)
    midi_recorded_legato: Option<String>,
    /// Base note name for the n-offset pattern
    midi_recorded_base_note: Option<String>,
    /// Number of cycles the pattern spans
    midi_recorded_cycles: usize,
    /// Counter for auto-generating ~rec1, ~rec2, etc.
    recording_counter: usize,
    /// Available MIDI input devices
    midi_devices: Vec<String>,
    /// MIDI quantization setting (0 = off, 4 = quarter notes, 8 = 8th, 16 = 16th, 32 = 32nd)
    midi_quantize: u8,
    /// Whether to show configuration panel
    show_config_panel: bool,
    /// Live recording preview line (displayed during recording)
    recording_preview_line: Option<String>,
    /// Currently held notes during recording (for live display)
    recording_held_notes: String,
    /// Vertical scroll offset (line number of first visible line)
    scroll_offset: u16,
    /// Last known viewport height (for scroll calculations)
    viewport_height: u16,
    /// Plugin browser panel
    plugin_browser: PluginBrowser,
    /// Plugin instance manager
    plugin_manager: PluginInstanceManager,
    /// Active VST3 GUI windows (plugin_name -> GUI handle)
    #[cfg(all(target_os = "linux", feature = "vst3"))]
    vst3_guis: HashMap<String, Vst3Gui>,
    /// Preview plugins loaded outside audio graph (for GUI browsing)
    #[cfg(all(target_os = "linux", feature = "vst3"))]
    preview_plugins: HashMap<String, crate::plugin_host::real_plugin::RealPluginInstance>,
    /// Last time parameter changes were polled (for throttling)
    #[cfg(all(target_os = "linux", feature = "vst3"))]
    last_param_poll: std::time::Instant,
}

impl ModalEditor {
    /// Create a new modal editor
    pub fn new(
        _duration: f32, // Deprecated parameter, kept for API compatibility
        file_path: Option<PathBuf>,
        buffer_size: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Buffer size from CLI arg, clamped to valid range (default 512)
        let synthesis_buffer_size = buffer_size.unwrap_or(512).clamp(64, 16384);

        // Suppress stderr output that would break the TUI
        // This includes: ALSA errors, X11 authorization messages, VST3 plugin output
        // NOTE: We only redirect stderr, NOT stdout - crossterm needs stdout for TUI!
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            if let Ok(log_file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/phonon_audio_errors.log")
            {
                let fd = log_file.as_raw_fd();
                unsafe {
                    // Only redirect stderr - stdout must remain connected to terminal for crossterm TUI
                    libc::dup2(fd, libc::STDERR_FILENO);
                    // DO NOT redirect stdout - it breaks crossterm!
                    // libc::dup2(fd, libc::STDOUT_FILENO);
                }
                // Keep log_file open by leaking it (intentional - we need the fd to stay valid)
                std::mem::forget(log_file);
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

        // Note: These messages go to log file now, not visible in TUI
        // eprintln!("🎵 Audio: {} Hz, {} channels, buffer: {} samples", sample_rate as u32, channels, synthesis_buffer_size);
        // eprintln!("🔧 Using ring buffer architecture for parallel synthesis");

        // Graph for background synthesis thread (lock-free swap)
        let graph = Arc::new(ArcSwap::from_pointee(None::<GraphCell>));

        // Underrun counter (shared between audio callback and UI)
        let underrun_count = Arc::new(AtomicUsize::new(0));

        // Performance monitoring (shared with synthesis thread)
        let synth_time_us = Arc::new(AtomicUsize::new(0));
        let ring_fill_percent = Arc::new(AtomicUsize::new(100));

        // Flag to signal audio callback to drain ring buffer on graph swap
        // This enables instant transitions without hearing stale audio
        let should_clear_ring = Arc::new(AtomicBool::new(false));

        // Ring buffer: background synth writes, audio callback reads
        // Size: ~200ms - balance between latency and cushion for variation
        // With sample preloading, we don't need a huge buffer for initialization spikes
        let ring_buffer_size = (sample_rate as usize / 5).max(4410); // ~200ms
        let ring = HeapRb::<f32>::new(ring_buffer_size);
        let (mut ring_producer, mut ring_consumer) = ring.split();

        // Background synthesis thread: continuously renders samples into ring buffer
        let graph_clone_synth = Arc::clone(&graph);
        let synth_time_us_clone = Arc::clone(&synth_time_us);
        let ring_fill_clone = Arc::clone(&ring_fill_percent);
        thread::spawn(move || {
            // Render in chunks of synthesis_buffer_size samples
            let mut buffer = vec![0.0f32; synthesis_buffer_size];
            let mut iterations = 0u64;
            let mut renders = 0u64;
            let mut sleeps = 0u64;
            let mut last_log = std::time::Instant::now();

            loop {
                iterations += 1;

                // Log stats every second to diagnose blocking
                if last_log.elapsed().as_secs() >= 1 {
                    // Calculate required renders for realtime: ~86 buffers/second (44100 / 512)
                    let required_renders = 86;
                    let status = if renders >= required_renders {
                        "✅"
                    } else {
                        "❌ UNDERRUN RISK"
                    };
                    eprintln!(
                        "🔧 Synth: {} renders/s (need {}) {} | {} iters, {} sleeps",
                        renders, required_renders, status, iterations, sleeps
                    );
                    iterations = 0;
                    renders = 0;
                    sleeps = 0;
                    last_log = std::time::Instant::now();
                }

                // Check if we have space in ring buffer
                let space = ring_producer.vacant_len();
                let total_size = ring_producer.capacity().get();
                let fill_percent = ((total_size - space) * 100) / total_size;
                ring_fill_clone.store(fill_percent, Ordering::Relaxed);

                if space >= buffer.len() {
                    // Render a chunk of audio
                    let graph_snapshot = graph_clone_synth.load();

                    if let Some(ref graph_cell) = **graph_snapshot {
                        // Measure synthesis performance
                        let start = std::time::Instant::now();

                        // Try to borrow - use try_borrow_mut to avoid panic!
                        match graph_cell.0.try_borrow_mut() {
                            Ok(mut graph) => {
                                // Synthesize samples using optimized buffer processing
                                graph.process_buffer(&mut buffer);
                                renders += 1;

                                let elapsed_us = start.elapsed().as_micros() as usize;
                                synth_time_us_clone.store(elapsed_us, Ordering::Relaxed);

                                // DEBUG: Track peak synthesis times
                                static MAX_SYNTH_US: std::sync::atomic::AtomicUsize =
                                    std::sync::atomic::AtomicUsize::new(0);
                                let prev_max = MAX_SYNTH_US.fetch_max(elapsed_us, Ordering::Relaxed);
                                if elapsed_us > prev_max && elapsed_us > 11610 {
                                    // Log when we exceed budget for first time at this level
                                    let voice_count = graph.active_voice_count();
                                    eprintln!(
                                        "🔥 NEW PEAK: {} us ({:.1}ms) - {}% budget | voices: {}",
                                        elapsed_us,
                                        elapsed_us as f64 / 1000.0,
                                        elapsed_us * 100 / 11610,
                                        voice_count
                                    );
                                }

                                // Write to ring buffer
                                let written = ring_producer.push_slice(&buffer);
                                if written < buffer.len() {
                                    eprintln!(
                                        "⚠️  Ring buffer full, dropped {} samples",
                                        buffer.len() - written
                                    );
                                }
                            }
                            Err(_) => {
                                // RefCell is borrowed - main thread is swapping graphs
                                // CRITICAL FIX: Don't write silence! Just skip this iteration.
                                // Missing one 512-sample chunk (11.6ms) won't cause underruns
                                // with our 100ms buffer, but writing silence causes harsh
                                // cutoffs during live code hot-swapping (C-x).
                                // The next iteration will use the new graph seamlessly.
                                use std::sync::atomic::{AtomicUsize, Ordering};
                                static SWAP_SKIP_COUNT: AtomicUsize = AtomicUsize::new(0);
                                let count = SWAP_SKIP_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                                if count % 10 == 1 {
                                    eprintln!("⚡ Skipped render during graph swap ({}x)", count);
                                }
                                // Don't increment renders counter, don't write to ring buffer
                                // Just continue to next iteration
                            }
                        }
                    } else {
                        // No graph (hushed/panic) - write silence
                        // CRITICAL: Fill buffer with zeros, don't reuse old audio!
                        buffer.fill(0.0);
                        let written = ring_producer.push_slice(&buffer);
                        synth_time_us_clone.store(0, Ordering::Relaxed);
                        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
                        static NO_GRAPH_COUNT: AtomicUsize = AtomicUsize::new(0);
                        let count = NO_GRAPH_COUNT.fetch_add(1, AtomicOrdering::Relaxed) + 1;
                        if count % 100 == 0 {
                            eprintln!("⚠️  No graph loaded! ({}x)", count);
                        }
                    }
                } else {
                    // Ring buffer is full, sleep briefly
                    thread::sleep(StdDuration::from_micros(100));
                    sleeps += 1;
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
                let _ = writeln!(
                    file,
                    "[{}] Audio stream error: {}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    err
                );
            }
        };

        // Clone underrun counter for audio callbacks
        let underrun_count_f32 = Arc::clone(&underrun_count);
        let underrun_count_i16 = Arc::clone(&underrun_count);

        // Clone clear flag for audio callbacks
        let should_clear_f32 = Arc::clone(&should_clear_ring);
        let should_clear_i16 = Arc::clone(&should_clear_ring);

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        // Check if we should clear the ring buffer (graph was swapped)
                        // This enables instant transitions without hearing stale audio
                        if should_clear_f32.swap(false, Ordering::Relaxed) {
                            // Drain all existing samples from the ring buffer
                            let to_drain = ring_consumer.occupied_len();
                            ring_consumer.skip(to_drain);
                        }

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

                            // Increment underrun counter (atomic, thread-safe)
                            underrun_count_f32.fetch_add(1, Ordering::Relaxed);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                // Pre-allocate conversion buffer OUTSIDE the callback to avoid
                // allocation in the realtime audio thread (critical for performance!)
                // Initial size 4096 handles most buffer sizes; resizes are rare and amortized
                let mut conversion_buffer: Vec<f32> = vec![0.0; 4096];

                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        // Check if we should clear the ring buffer (graph was swapped)
                        // This enables instant transitions without hearing stale audio
                        if should_clear_i16.swap(false, Ordering::Relaxed) {
                            // Drain all existing samples from the ring buffer
                            let to_drain = ring_consumer.occupied_len();
                            ring_consumer.skip(to_drain);
                        }

                        let available = ring_consumer.occupied_len();

                        // Ensure conversion buffer is large enough (rare resize, amortized)
                        if conversion_buffer.len() < data.len() {
                            conversion_buffer.resize(data.len(), 0.0);
                        }

                        if available >= data.len() {
                            // Read from ring buffer and convert to i16
                            // Use pre-allocated buffer slice - NO ALLOCATION!
                            let temp = &mut conversion_buffer[..data.len()];
                            ring_consumer.pop_slice(temp);
                            for (dst, src) in data.iter_mut().zip(temp.iter()) {
                                *dst = (*src * 32767.0) as i16;
                            }
                        } else {
                            // Underrun - read what's available
                            if available > 0 {
                                let temp = &mut conversion_buffer[..available];
                                ring_consumer.pop_slice(temp);
                                for (i, dst) in data.iter_mut().enumerate() {
                                    if i < available {
                                        *dst = (temp[i] * 32767.0) as i16;
                                    } else {
                                        *dst = 0;
                                    }
                                }
                            } else {
                                // No samples at all, fill with silence
                                for dst in data.iter_mut() {
                                    *dst = 0;
                                }
                            }

                            // Increment underrun counter (atomic, thread-safe)
                            underrun_count_i16.fetch_add(1, Ordering::Relaxed);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => return Err("Unsupported sample format".into()),
        }
        .map_err(|e| format!("Failed to build stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to play stream: {}", e))?;

        // Load initial content
        let content = if let Some(ref path) = file_path {
            if path.exists() {
                fs::read_to_string(path)?
            } else {
                String::new()
            }
        } else {
            // Default starter template
            String::from("-- Phonon Live Coding\n-- C-x: Eval block | C-l: Reload all | C-h: Hush | C-s: Save | Alt-q: Quit\n\n-- Example: Simple drum pattern\ntempo: 0.5\n~drums $ s \"bd sn bd sn\"\nout $ ~drums * 0.8\n")
        };

        // Start cursor at beginning of file (not end)
        let cursor_pos = 0;
        let bus_names = completion::extract_bus_names(&content);

        // Create editor instance first
        let mut editor = Self {
            cursor_pos,
            content,
            file_path,
            status_message:
                "🎵 Ready - C-x: eval block | C-l: reload all | C-u: undo | C-r: redo | Alt-/: help"
                    .to_string(),
            is_playing: false,
            error_message: None,
            graph,
            _stream: Some(stream),
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
            underrun_count,
            synth_time_us,
            ring_fill_percent,
            should_clear_ring,
            midi_input: None,
            midi_recorder: None,
            midi_recording: false,
            midi_recorded_pattern: None,
            midi_recorded_n_pattern: None,
            midi_recorded_velocity: None,
            midi_recorded_legato: None,
            midi_recorded_base_note: None,
            midi_recorded_cycles: 0,
            recording_counter: 0,
            midi_devices: MidiInputHandler::list_devices()
                .unwrap_or_default()
                .into_iter()
                .map(|d| d.name)
                .collect(),
            midi_quantize: 16, // Default to 16th note quantization
            show_config_panel: false,
            recording_preview_line: None,
            recording_held_notes: String::new(),
            scroll_offset: 0,
            viewport_height: 20,
            plugin_browser: PluginBrowser::new(),
            plugin_manager: PluginInstanceManager::new(),
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            vst3_guis: HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            preview_plugins: HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            last_param_poll: std::time::Instant::now(),
        };

        // Initialize plugin manager
        let _ = editor.plugin_manager.initialize(sample_rate, synthesis_buffer_size);
        let _ = editor.plugin_manager.registry_mut().scan();

        // Auto-connect to first MIDI device if available
        editor.auto_connect_midi();

        Ok(editor)
    }

    /// Create a headless editor for testing (no audio device required)
    /// This allows running editor tests in CI environments without audio hardware
    pub fn new_headless() -> Result<Self, Box<dyn std::error::Error>> {
        let sample_rate = 44100.0;
        let graph = Arc::new(ArcSwap::from_pointee(None::<GraphCell>));
        let underrun_count = Arc::new(AtomicUsize::new(0));
        let synth_time_us = Arc::new(AtomicUsize::new(0));
        let ring_fill_percent = Arc::new(AtomicUsize::new(100));
        let should_clear_ring = Arc::new(AtomicBool::new(false));

        let content = String::new();
        let bus_names = completion::extract_bus_names(&content);

        Ok(Self {
            cursor_pos: 0,
            content,
            file_path: None,
            status_message: "Headless test mode".to_string(),
            is_playing: false,
            error_message: None,
            graph,
            _stream: None, // No audio stream in headless mode
            sample_rate,
            flash_highlight: None,
            kill_buffer: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            console_messages: Vec::new(),
            completion_state: completion::CompletionState::new(),
            sample_names: completion::discover_samples(),
            bus_names,
            command_console: CommandConsole::new(),
            underrun_count,
            synth_time_us,
            ring_fill_percent,
            should_clear_ring,
            midi_input: None,
            midi_recorder: None,
            midi_recording: false,
            midi_recorded_pattern: None,
            midi_recorded_n_pattern: None,
            midi_recorded_velocity: None,
            midi_recorded_legato: None,
            midi_recorded_base_note: None,
            midi_recorded_cycles: 0,
            recording_counter: 0,
            midi_devices: Vec::new(),
            midi_quantize: 16,
            show_config_panel: false,
            recording_preview_line: None,
            recording_held_notes: String::new(),
            scroll_offset: 0,
            viewport_height: 20,
            plugin_browser: PluginBrowser::new(),
            plugin_manager: PluginInstanceManager::new(),
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            vst3_guis: HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            preview_plugins: HashMap::new(),
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            last_param_poll: std::time::Instant::now(),
        })
    }

    /// Load and compile DSL code into the audio graph
    fn load_code(&mut self, code: &str) -> Result<(), String> {
        eprintln!("🔧 load_code() called with {} bytes", code.len());

        // Parse the DSL code
        let (rest, statements) = parse_program(code).map_err(|e| {
            eprintln!("❌ Parse error: {}", e);
            format!("Parse error: {}", e)
        })?;

        if !rest.trim().is_empty() {
            let err = format!("Failed to parse entire code, remaining: {}", rest);
            eprintln!("❌ {}", err);
            return Err(err);
        }

        eprintln!("✅ Parsed {} statements", statements.len());

        // Compile into a graph
        // Note: compile_program sets CPS from tempo:/bpm: statements in the code
        // Default is 0.5 CPS if not specified
        // Pass MIDI event queue for real-time monitoring (~midi buses)
        let midi_queue = self
            .midi_input
            .as_ref()
            .map(|handler| handler.get_monitoring_queue());

        let mut new_graph =
            compile_program(statements, self.sample_rate, midi_queue).map_err(|e| {
                eprintln!("❌ Compile error: {}", e);
                format!("Compile error: {}", e)
            })?;

        eprintln!("✅ Compiled graph successfully");
        eprintln!("📊 New graph CPS from code: {}", new_graph.get_cps());

        // CRITICAL: Check if we have an old graph to transfer timing from
        // If we do, transfer will preserve wall-clock timing
        // If we don't (first load), we need to initialize wall-clock timing
        let has_old_graph = matches!(**self.graph.load(), Some(_));

        // Log new graph's CPS BEFORE any modifications
        eprintln!("📊 New graph compiled with CPS: {}", new_graph.get_cps());

        // ALWAYS enable wall-clock timing for live mode
        // This must happen BEFORE transfer_session_timing (which also sets use_wall_clock=true)
        // but we need a valid session_start_time even if transfer fails
        new_graph.enable_wall_clock_timing();

        if !has_old_graph {
            eprintln!("📊 First load - wall-clock timing initialized");
        }

        // CRITICAL: Transfer state from old graph to prevent clicks and timing shifts
        // This ensures seamless hot-swapping:
        // 1. Session timing transferred → global clock never drops the beat!
        // 2. VoiceManager transferred → active voices continue playing → no click!
        //
        // Use try_borrow() with brief sleeps to avoid spinning CPU
        let current_graph = self.graph.load();
        eprintln!("📊 Current graph exists: {}", current_graph.is_some());
        if let Some(ref old_graph_cell) = **current_graph {
            let mut state_transferred = false;

            // Retry with small sleeps - don't burn CPU with spin-loop!
            // Audio buffer at 512 samples @ 44.1kHz = ~11.6ms per buffer
            // We retry for ~25ms total (50 attempts × 0.5ms) to handle worst-case timing
            for attempt in 0..50 {
                match old_graph_cell.0.try_borrow_mut() {
                    Ok(mut old_graph) => {
                        eprintln!("📊 Transfer succeeded on attempt {}", attempt);
                        eprintln!(
                            "📊 Before transfer - old graph cycle position: {}",
                            old_graph.get_cycle_position()
                        );
                        eprintln!(
                            "📊 Before transfer - old graph CPS: {}",
                            old_graph.get_cps()
                        );

                        // CRITICAL: Transfer session timing (wall-clock based)
                        // This preserves the global clock - beat NEVER drops!
                        new_graph.transfer_session_timing(&old_graph);

                        eprintln!(
                            "📊 After transfer - new graph cycle position: {}",
                            new_graph.get_cycle_position()
                        );
                        eprintln!("📊 After transfer - new graph CPS: {}", new_graph.get_cps());

                        // CRITICAL: Transfer FX state (delay buffers, reverb tails, etc.)
                        // This preserves audio continuity - FX tails don't cut off!
                        new_graph.transfer_fx_states(&old_graph);

                        // CRITICAL: Transfer VoiceManager to preserve active voices!
                        // This prevents the click from voices being cut off mid-sample
                        new_graph.transfer_voice_manager(old_graph.take_voice_manager());

                        state_transferred = true;
                        break;
                    }
                    Err(_) => {
                        // Audio thread busy processing - sleep briefly and retry
                        // 500 microseconds = 0.5ms, small enough to feel instant but prevents CPU burn
                        std::thread::sleep(std::time::Duration::from_micros(500));
                    }
                }
            }

            if !state_transferred {
                // Still couldn't get it after 10ms of retries - very rare, audio thread might be stuck
                // CRITICAL: Even without transfer, we need valid timing!
                // The new graph already has wall-clock enabled (from above), so it will
                // use its own session_start_time. This means timing will restart from 0,
                // which may cause a beat jump, but at least tempo won't double.
                eprintln!("⚠️  Could not transfer state after retries!");
                eprintln!("   New graph starting with fresh timing (beat may jump)");
                eprintln!(
                    "   New graph CPS: {}, wall-clock: {}",
                    new_graph.get_cps(),
                    new_graph.use_wall_clock
                );
            }
        }

        // CRITICAL: Preload all samples BEFORE swapping graph into audio thread
        // This prevents disk I/O during audio processing (which causes underruns)
        new_graph.preload_samples();

        // Hot-swap the graph atomically using lock-free ArcSwap
        // Background synthesis thread will pick up new graph on next render
        self.graph
            .store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));

        // DON'T clear the ring buffer for live coding!
        // Let it play out smoothly - the new graph will naturally take over.
        // This prevents beat drops and maintains groove continuity.
        // (Only clear ring for explicit "hush" command)

        eprintln!("✅ Graph stored! Smooth transition to new code...");

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

            // Process any pending MIDI input events
            self.process_midi_events();

            // Pump VST3 GUI events and cleanup closed windows (Linux only, with vst3 feature)
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            {
                // Pump events for all GUIs
                for gui in self.vst3_guis.values_mut() {
                    gui.pump_events();
                }
                // Remove closed GUIs so they can be reopened
                self.vst3_guis.retain(|_name, gui| gui.is_visible());
            }

            // Poll for parameter changes from VST3 GUIs (Linux only)
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            if !self.vst3_guis.is_empty() {
                self.poll_vst3_param_changes();
            }

            // Update recording status with cycle position
            if self.midi_recording {
                self.update_recording_status();
            }

            terminal.draw(|f| self.ui(f))?;

            // Use poll with timeout to enable flash animation
            // 100ms = reduced refresh rate (was 50ms) for less CPU usage
            if event::poll(std::time::Duration::from_millis(100))? {
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

        // If plugin browser is visible, route keys to it
        if self.plugin_browser.is_visible() {
            return self.handle_plugin_browser_key_event(key);
        }

        // If config panel is visible, handle config keys
        if self.show_config_panel {
            match key.code {
                // Q: Cycle quantization setting
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.cycle_quantization();
                    return KeyResult::Continue;
                }
                // Esc: Close config panel
                KeyCode::Esc => {
                    self.show_config_panel = false;
                    self.status_message = "Configuration closed".to_string();
                    return KeyResult::Continue;
                }
                // Alt+Comma: Toggle off
                KeyCode::Char(',') if key.modifiers.contains(KeyModifiers::ALT) => {
                    self.show_config_panel = false;
                    self.status_message = "Configuration closed".to_string();
                    return KeyResult::Continue;
                }
                // Other keys ignored in config mode
                _ => return KeyResult::Continue,
            }
        }

        match key.code {
            // Quit with Alt+Q (Ctrl+Q conflicts with terminal flow control)
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::ALT) => KeyResult::Quit,

            // Alt+/ : Toggle command console
            KeyCode::Char('/') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.command_console.toggle();
                KeyResult::Continue
            }

            // Alt+P: Toggle plugin browser
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.plugin_browser.toggle();
                KeyResult::Continue
            }

            // Alt+G: Open VST3 plugin GUIs
            #[cfg(all(target_os = "linux", feature = "vst3"))]
            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.open_plugin_guis();
                KeyResult::Continue
            }

            // Ctrl+X: Evaluate current block (chunk)
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.eval_chunk();
                KeyResult::Continue
            }

            // Ctrl+L: Reload all (evaluate entire buffer)
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.eval_all();
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

            // Alt+M: Connect to MIDI device (cycles through available devices)
            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.cycle_midi_device();
                KeyResult::Continue
            }

            // Alt+Comma: Toggle configuration panel
            KeyCode::Char(',') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.show_config_panel = !self.show_config_panel;
                if self.show_config_panel {
                    self.status_message =
                        "⚙️  Configuration Panel (Q: quantize, Esc: close)".to_string();
                } else {
                    self.status_message = "Configuration closed".to_string();
                }
                KeyResult::Continue
            }

            // Alt+R: Start/stop MIDI recording
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.toggle_midi_recording();
                KeyResult::Continue
            }

            // Alt+Shift+I: Smart paste complete pattern (~rec1: slow N $ n "..." # gain "...")
            KeyCode::Char('I')
                if key.modifiers.contains(KeyModifiers::ALT)
                    && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.insert_midi_smart_paste();
                KeyResult::Continue
            }

            // Alt+I: Insert recorded MIDI pattern at cursor (note names)
            KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.insert_midi_pattern();
                KeyResult::Continue
            }

            // Alt+N: Insert recorded MIDI pattern as n-offsets
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.insert_midi_n_pattern();
                KeyResult::Continue
            }

            // Alt+V: Insert recorded MIDI velocity pattern
            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.insert_midi_velocity_pattern();
                KeyResult::Continue
            }

            // Alt+L: Insert recorded MIDI legato pattern
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.insert_midi_legato_pattern();
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

            // Ctrl+Space: Accept completion with defaults, or expand kwargs at cursor
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.completion_state.is_visible() {
                    // Accept current completion with default parameters
                    self.accept_completion_with_defaults();
                } else {
                    // Expand kwargs for function at cursor
                    self.expand_kwargs_template();
                }
                KeyResult::Continue
            }

            // Tab: trigger completion or accept current selection
            // Shift+Tab: expand function with all kwargs and defaults
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Shift+Tab: expand kwargs template
                    self.expand_kwargs_template();
                } else if self.completion_state.is_visible() {
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
                // '?' toggles docs panel when completion is visible
                if c == '?' && self.completion_state.is_visible() {
                    self.completion_state.toggle_docs_panel();
                    return KeyResult::Continue;
                }

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
            // F1 toggles docs panel when completion is visible
            KeyCode::F(1) => {
                if self.completion_state.is_visible() {
                    self.completion_state.toggle_docs_panel();
                }
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

        // Update viewport height and ensure cursor is visible
        self.viewport_height = editor_chunk.height;
        self.ensure_cursor_visible();

        let content_with_cursor = self.content_with_cursor();
        let paragraph = Paragraph::new(content_with_cursor)
            .block(editor_block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0))
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(paragraph, editor_chunk);

        // Completion popup (if active)
        if self.completion_state.is_visible() {
            let completions = self.completion_state.completions();
            let selected_index = self.completion_state.selected_index();
            let show_docs = self.completion_state.is_docs_panel_visible();

            let popup_width = 55;
            let popup_y = 3;

            // Calculate max available height
            let max_available = editor_chunk.height.saturating_sub(popup_y + 1) as usize;

            // Completion list height
            let list_content_height = completions.len().min(8); // Max 8 visible items
            let list_height = (list_content_height + 2) as u16; // +2 for borders

            // Docs panel height (if showing)
            let docs_height = if show_docs {
                if let Some(selected) = self.completion_state.selected_completion() {
                    if let Some(docs) = completion::FunctionDocs::get(&selected.text) {
                        let doc_lines = docs.format_lines(popup_width as usize - 4);
                        (doc_lines.len() + 3).min(10) as u16 // +3 for borders and padding, max 10
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            };

            // Total popup height
            let total_height = (list_height + docs_height).min(max_available as u16);

            // Position popup near cursor
            let popup_x = editor_chunk.width.saturating_sub(popup_width + 2).max(2);

            // Calculate scroll offset to keep selected item visible
            let visible_items = list_content_height;
            let scroll_offset = if selected_index < visible_items {
                0
            } else {
                let ideal_offset = selected_index.saturating_sub(visible_items / 2);
                let max_offset = completions.len().saturating_sub(visible_items);
                ideal_offset.min(max_offset)
            };

            // Build completion list content
            let mut popup_lines = Vec::new();

            // Show scroll indicator at top if there are items above
            if scroll_offset > 0 {
                popup_lines.push(Line::from(Span::styled(
                    "  ▲ more above ▲",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            let visible_completions = completions.iter().skip(scroll_offset).take(visible_items);

            for (displayed_idx, compl) in visible_completions.enumerate() {
                let actual_idx = scroll_offset + displayed_idx;
                let is_selected = actual_idx == selected_index;
                let prefix = if is_selected { "► " } else { "  " };
                let base_style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                let highlight_style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                };

                // Build spans with highlighted matched characters
                let mut spans = vec![Span::styled(prefix, base_style)];

                let text_chars: Vec<char> = compl.text.chars().collect();
                let matched_set: std::collections::HashSet<usize> =
                    compl.matched_indices.iter().copied().collect();

                for (i, ch) in text_chars.iter().enumerate() {
                    let style = if matched_set.contains(&i) {
                        highlight_style
                    } else {
                        base_style
                    };
                    spans.push(Span::styled(ch.to_string(), style));
                }

                // Pad to 20 chars for alignment
                let padding_needed = 20usize.saturating_sub(compl.text.len());
                if padding_needed > 0 {
                    spans.push(Span::styled(" ".repeat(padding_needed), base_style));
                }

                // Add the type label
                spans.push(Span::styled(format!(" {}", compl.label()), base_style));

                popup_lines.push(Line::from(spans));
            }

            // Show scroll indicator at bottom if there are items below
            if scroll_offset + visible_items < completions.len() {
                popup_lines.push(Line::from(Span::styled(
                    "  ▼ more below ▼",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            // Render completion list
            let list_area = ratatui::layout::Rect {
                x: editor_chunk.x + popup_x,
                y: editor_chunk.y + popup_y,
                width: popup_width,
                height: list_height.min(total_height),
            };

            let title = if show_docs {
                "Completions [? toggle docs]"
            } else {
                "Completions [? show docs]"
            };

            let popup_block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan).bg(Color::Black));

            let popup_paragraph = Paragraph::new(popup_lines)
                .block(popup_block)
                .style(Style::default().bg(Color::Black));

            f.render_widget(popup_paragraph, list_area);

            // Render docs panel below the completion list (if visible and we have docs)
            if show_docs && docs_height > 0 {
                if let Some(selected) = self.completion_state.selected_completion() {
                    if let Some(docs) = completion::FunctionDocs::get(&selected.text) {
                        let doc_lines = docs.format_lines(popup_width as usize - 4);

                        let docs_area = ratatui::layout::Rect {
                            x: editor_chunk.x + popup_x,
                            y: editor_chunk.y + popup_y + list_height,
                            width: popup_width,
                            height: docs_height,
                        };

                        let mut styled_lines: Vec<Line> = Vec::new();

                        for doc_line in doc_lines.iter().take(docs_height as usize - 2) {
                            let style = match doc_line.style {
                                completion::DocLineStyle::Header => Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(ratatui::style::Modifier::BOLD),
                                completion::DocLineStyle::Subheader => {
                                    Style::default().fg(Color::Yellow)
                                }
                                completion::DocLineStyle::Param => Style::default().fg(Color::White),
                                completion::DocLineStyle::Example => {
                                    Style::default().fg(Color::Green)
                                }
                                completion::DocLineStyle::Empty => Style::default(),
                            };
                            styled_lines.push(Line::from(Span::styled(&doc_line.text, style)));
                        }

                        let docs_block = Block::default()
                            .title(format!("{} [{}]", docs.name, docs.category))
                            .borders(Borders::ALL)
                            .style(Style::default().fg(Color::Magenta).bg(Color::Black));

                        let docs_paragraph = Paragraph::new(styled_lines)
                            .block(docs_block)
                            .style(Style::default().bg(Color::Black));

                        f.render_widget(docs_paragraph, docs_area);
                    }
                }
            }
        }

        // Configuration panel (if visible)
        if self.show_config_panel {
            let quantize_str = match self.midi_quantize {
                0 => "Off (no quantization)",
                4 => "Quarter notes (4 per cycle)",
                8 => "8th notes (8 per cycle)",
                16 => "16th notes (16 per cycle)",
                32 => "32nd notes (32 per cycle)",
                _ => "Unknown",
            };

            let midi_device_str = if self.midi_input.is_some() {
                "Connected"
            } else {
                "Not connected"
            };

            let config_lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  [Q] Quantization: ", Style::default().fg(Color::Yellow)),
                    Span::raw(quantize_str),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  [M] MIDI Device: ", Style::default().fg(Color::Yellow)),
                    Span::raw(midi_device_str),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  [Esc] ", Style::default().fg(Color::Cyan)),
                    Span::raw("Close"),
                ]),
                Line::from(""),
            ];

            let popup_width = 50;
            let popup_height = 9;
            let popup_x = (editor_chunk.width.saturating_sub(popup_width)) / 2;
            let popup_y = (editor_chunk.height.saturating_sub(popup_height)) / 2;

            let config_area = ratatui::layout::Rect {
                x: editor_chunk.x + popup_x,
                y: editor_chunk.y + popup_y,
                width: popup_width,
                height: popup_height,
            };

            let config_block = Block::default()
                .title(" ⚙️  Recording Configuration ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Green).bg(Color::Black));

            let config_paragraph = Paragraph::new(config_lines)
                .block(config_block)
                .style(Style::default().bg(Color::Black));

            f.render_widget(config_paragraph, config_area);
        }

        // Recording preview overlay (shown during MIDI recording)
        if self.midi_recording {
            if let Some(ref preview_line) = self.recording_preview_line {
                // Calculate preview area - bottom of editor, spanning full width
                let preview_height = 3u16;
                let preview_y = editor_chunk.y + editor_chunk.height.saturating_sub(preview_height + 1);

                let preview_area = ratatui::layout::Rect {
                    x: editor_chunk.x + 1,
                    y: preview_y,
                    width: editor_chunk.width.saturating_sub(2),
                    height: preview_height,
                };

                // Build preview content with held notes indicator
                let held_indicator = if !self.recording_held_notes.is_empty() {
                    format!("  ♪ {}", self.recording_held_notes)
                } else {
                    String::new()
                };

                let preview_content = format!(
                    "{}{}",
                    preview_line,
                    held_indicator
                );

                let preview_block = Block::default()
                    .title(" 🔴 RECORDING ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
                    .style(Style::default().bg(Color::Black));

                let preview_paragraph = Paragraph::new(preview_content)
                    .block(preview_block)
                    .style(Style::default().fg(Color::Yellow).bg(Color::Black));

                f.render_widget(preview_paragraph, preview_area);
            }
        }

        // Status area with performance stats
        let underrun_count = self.underrun_count.load(Ordering::Relaxed);
        let synth_time_us = self.synth_time_us.load(Ordering::Relaxed);
        let ring_fill = self.ring_fill_percent.load(Ordering::Relaxed);

        // Calculate synthesis performance
        // 512 samples @ 44.1kHz = 11,610 microseconds per buffer (realtime budget)
        let budget_us = 11_610;
        let synth_percent = if synth_time_us > 0 {
            (synth_time_us * 100) / budget_us
        } else {
            0
        };

        // Determine if synthesis is currently too slow (not historical underruns)
        let is_too_slow = synth_percent > 100;

        let status_style = if self.error_message.is_some() {
            Style::default().fg(Color::Red)
        } else if is_too_slow {
            // Only red if CURRENTLY slow, not for historical underruns
            Style::default().fg(Color::Red)
        } else if self.is_playing {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let status_text = if let Some(ref error) = self.error_message {
            if underrun_count > 0 {
                format!(
                    "❌ Error: {error} | Underruns: {} (total) | Synth: {}% | Buf: {}%",
                    underrun_count, synth_percent, ring_fill
                )
            } else {
                format!("❌ Error: {error}")
            }
        } else if synth_time_us > 0 {
            // Show detailed performance info with underrun history (not alarming)
            let perf_status = if is_too_slow {
                "⚠️ TOO SLOW!"
            } else {
                "✓"
            };
            format!(
                "🔊 {} Synth: {}% ({}/{}µs) | Buf: {}% | Underruns: {} (total)",
                perf_status, synth_percent, synth_time_us, budget_us, ring_fill, underrun_count
            )
        } else if self.is_playing {
            format!("🔊 Playing... | Underruns: {} (total)", underrun_count)
        } else {
            format!(
                "{} | Underruns: {} (total)",
                self.status_message, underrun_count
            )
        };

        let help_text = "C-x: Eval block | C-l: Reload all | C-u: Undo | C-r: Redo | C-h: Hush | C-s: Save | Alt-q: Quit";

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

        // Plugin browser overlay (rendered on top of everything)
        if self.plugin_browser.is_visible() {
            // Create centered popup area (70% width, 70% height)
            let area = f.size();
            let popup_width = (area.width as f32 * 0.7) as u16;
            let popup_height = (area.height as f32 * 0.7) as u16;
            let popup_x = (area.width - popup_width) / 2;
            let popup_y = (area.height - popup_height) / 2;

            let popup_area = ratatui::layout::Rect {
                x: popup_x,
                y: popup_y,
                width: popup_width,
                height: popup_height,
            };

            self.plugin_browser.render(f, popup_area, &self.plugin_manager);
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
    fn content_with_cursor(&self) -> Vec<Line<'_>> {
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

        // Evaluate ONLY the current chunk (Tidal-style block evaluation)
        // Use C-r to reload the entire buffer if needed
        let result = self.load_code(&chunk);

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
        // Clear ring buffer for instant silence
        self.should_clear_ring.store(true, Ordering::Relaxed);
        self.status_message = "🔇 Hushed - C-r to reload".to_string();
    }

    /// Panic - stop everything
    fn panic(&mut self) {
        // Clear the graph to stop everything
        self.graph.store(Arc::new(None));
        // Clear ring buffer for instant silence
        self.should_clear_ring.store(true, Ordering::Relaxed);
        self.status_message = "🚨 PANIC! All stopped - C-r to restart".to_string();
    }

    // ==================== MIDI INPUT ====================

    /// Auto-connect to the first available MIDI device on startup
    fn auto_connect_midi(&mut self) {
        // Refresh device list
        self.midi_devices = MidiInputHandler::list_devices()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.name)
            .collect();

        if self.midi_devices.is_empty() {
            // No devices - that's fine, user can connect later
            return;
        }

        // Prefer real hardware over virtual ports like "Midi Through"
        // Virtual ports are useful for routing but we want actual keyboards
        let device_name = self.midi_devices
            .iter()
            .find(|name| {
                let lower = name.to_lowercase();
                !lower.contains("midi through") && !lower.contains("virtual")
            })
            .cloned()
            .unwrap_or_else(|| self.midi_devices[0].clone());
        match MidiInputHandler::new() {
            Ok(mut handler) => {
                if let Err(e) = handler.connect(&device_name) {
                    eprintln!("🎹 MIDI auto-connect failed: {}", e);
                } else {
                    eprintln!("🎹 MIDI auto-connected: {}", device_name);
                    self.midi_input = Some(handler);
                    self.status_message = format!("🎹 MIDI: {} (Alt+R to record)", device_name);
                }
            }
            Err(e) => {
                eprintln!("🎹 MIDI init failed: {}", e);
            }
        }
    }

    /// Cycle through available MIDI input devices
    fn cycle_midi_device(&mut self) {
        // Refresh device list
        self.midi_devices = MidiInputHandler::list_devices()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.name)
            .collect();

        if self.midi_devices.is_empty() {
            self.status_message = "🎹 No MIDI devices found".to_string();
            return;
        }

        // Check current connection state
        let current_device = if let Some(ref handler) = self.midi_input {
            if handler.is_connected() {
                // Find which device we're connected to
                self.midi_devices.iter().position(|_| true).unwrap_or(0)
            } else {
                self.midi_devices.len() // Will wrap to 0
            }
        } else {
            self.midi_devices.len() // Will wrap to 0
        };

        // Cycle to next device
        let next_index = (current_device + 1) % (self.midi_devices.len() + 1);

        if next_index >= self.midi_devices.len() {
            // Disconnect
            self.midi_input = None;
            self.status_message = "🎹 MIDI disconnected".to_string();
        } else {
            // Connect to device
            let device_name = self.midi_devices[next_index].clone();
            match MidiInputHandler::new() {
                Ok(mut handler) => {
                    if let Err(e) = handler.connect(&device_name) {
                        self.status_message = format!("🎹 MIDI error: {}", e);
                    } else {
                        self.status_message = format!("🎹 MIDI: {}", device_name);
                        self.midi_input = Some(handler);
                    }
                }
                Err(e) => {
                    self.status_message = format!("🎹 MIDI init error: {}", e);
                }
            }
        }
    }

    /// Toggle MIDI recording on/off
    fn toggle_midi_recording(&mut self) {
        if self.midi_input.is_none() {
            // Try auto-connect first
            self.auto_connect_midi();
            if self.midi_input.is_none() {
                self.status_message = "🎹 No MIDI device found (Alt+M to refresh)".to_string();
                return;
            }
        }

        if self.midi_recording {
            // Stop recording
            self.midi_recording = false;
            self.recording_preview_line = None;
            self.recording_held_notes.clear();

            // Extract all data from recorder first (before any mutable borrows)
            let recording_data = if let Some(ref recorder) = self.midi_recorder {
                let beats_per_cycle = 4.0;
                recorder.to_recorded_pattern(beats_per_cycle).map(|recorded| {
                    let summary = recorder.get_recording_summary(beats_per_cycle);
                    (recorded, summary)
                })
            } else {
                None
            };

            // Now process the extracted data (recorder borrow is dropped)
            if let Some((recorded, summary)) = recording_data {
                // Store for manual insertion if needed
                self.midi_recorded_pattern = Some(recorded.notes.clone());
                self.midi_recorded_n_pattern = Some(recorded.n_offsets.clone());
                self.midi_recorded_velocity = Some(recorded.velocities.clone());
                self.midi_recorded_legato = Some(recorded.legato.clone());
                self.midi_recorded_base_note = Some(recorded.base_note_name.clone());
                self.midi_recorded_cycles = recorded.cycle_count;

                // Increment counter for next recording
                self.recording_counter += 1;
                let bus_name = format!("~rec{}", self.recording_counter);

                // Generate full code line with slow wrapper if needed
                let slow_wrapper = if recorded.cycle_count > 1 {
                    format!("slow {} $ ", recorded.cycle_count)
                } else {
                    String::new()
                };

                let code_line = format!(
                    "{} $ {}n \"{}\"",
                    bus_name, slow_wrapper, recorded.notes
                );

                // Ensure we're at a new line
                if self.cursor_pos > 0 {
                    let before_cursor = &self.content[..self.cursor_pos];
                    if !before_cursor.ends_with('\n') {
                        self.insert_char('\n');
                    }
                }

                // Insert the code line
                for c in code_line.chars() {
                    self.insert_char(c);
                }
                self.insert_char('\n');

                // Add to console
                self.add_console_message(&format!("📝 Recorded: {}", code_line));

                // Auto-execute the recorded pattern immediately
                self.eval_chunk();

                // Update status
                self.status_message = format!(
                    "🎵 {} playing as {}",
                    summary, bus_name
                );
            } else {
                self.status_message = "⏹️ Recording stopped (no notes)".to_string();
            }
        } else {
            // Start recording
            self.midi_recording = true;

            // Get tempo from current graph or use default 120 BPM
            let tempo = 120.0; // TODO: Get from graph.get_cps() * 60

            // Get current cycle position from graph (for punch-in)
            let current_cycle = {
                let graph_arc = self.graph.load();
                if let Some(ref graph_cell) = **graph_arc {
                    if let Ok(graph) = graph_cell.0.try_borrow() {
                        graph.get_cycle_position()
                    } else {
                        0.0 // Fallback if graph is borrowed
                    }
                } else {
                    0.0 // No graph yet
                }
            };

            self.midi_recorder = Some(MidiRecorder::new(tempo));
            if let Some(ref mut recorder) = self.midi_recorder {
                // Set quantization from config
                if self.midi_quantize > 0 {
                    recorder.set_quantize(self.midi_quantize);
                }

                // Use punch-in recording (start at current cycle)
                recorder.start_at_cycle(current_cycle);
            }

            self.status_message = format!(
                "⏺️ Recording MIDI at cycle {:.2}... (Alt+R to stop)",
                current_cycle
            );
        }
    }

    /// Cycle through quantization settings (0 = off, 4, 8, 16, 32)
    fn cycle_quantization(&mut self) {
        self.midi_quantize = match self.midi_quantize {
            0 => 4,   // off → quarter notes
            4 => 8,   // quarter → 8th notes
            8 => 16,  // 8th → 16th notes
            16 => 32, // 16th → 32nd notes
            32 => 0,  // 32nd → off
            _ => 16,  // default to 16th notes
        };

        let quantize_str = match self.midi_quantize {
            0 => "Off (no quantization)".to_string(),
            4 => "Quarter notes (4 per cycle)".to_string(),
            8 => "8th notes (8 per cycle)".to_string(),
            16 => "16th notes (16 per cycle)".to_string(),
            32 => "32nd notes (32 per cycle)".to_string(),
            _ => "Unknown".to_string(),
        };

        self.status_message = format!("⚙️  Quantization: {}", quantize_str);
    }

    /// Update recording status with current cycle position and live preview
    fn update_recording_status(&mut self) {
        if let Some(ref recorder) = self.midi_recorder {
            // Get current cycle position and beats_per_cycle from graph
            let (current_cycle, beats_per_cycle) = {
                let graph_arc = self.graph.load();
                if let Some(ref graph_cell) = **graph_arc {
                    if let Ok(graph) = graph_cell.0.try_borrow() {
                        (graph.get_cycle_position(), 4.0) // Default 4 beats per cycle
                    } else {
                        return; // Can't get cycle if graph is borrowed
                    }
                } else {
                    return; // No graph yet
                }
            };

            // Generate live preview using the new methods
            let preview = recorder.live_preview(beats_per_cycle);
            let bus_name = format!("~rec{}", self.recording_counter + 1);
            let code_preview = recorder.generate_code_preview(beats_per_cycle, &bus_name);

            // Store preview line for UI display
            self.recording_preview_line = Some(code_preview.clone());
            self.recording_held_notes = preview.currently_held.clone();

            // Build status message with cycle info and held notes
            let held_display = if preview.currently_held.is_empty() {
                String::new()
            } else {
                format!(" | Playing: {}", preview.currently_held)
            };

            let slow_indicator = if preview.total_cycles > 1 {
                format!(" (slow {})", preview.total_cycles)
            } else {
                String::new()
            };

            self.status_message = format!(
                "🔴 REC cycle {} | {} notes{}{} | Alt+R stop",
                preview.current_cycle,
                preview.note_count,
                slow_indicator,
                held_display
            );
        }
    }

    /// Insert recorded MIDI pattern at cursor position (note names)
    fn insert_midi_pattern(&mut self) {
        if let Some(ref pattern) = self.midi_recorded_pattern.clone() {
            // Insert the pattern as a quoted string
            let pattern_str = format!("\"{}\"", pattern);
            for c in pattern_str.chars() {
                self.insert_char(c);
            }
            self.status_message = format!("📝 Inserted: {}", pattern_str);
        } else {
            self.status_message = "🎹 No recorded pattern (Alt+R to record)".to_string();
        }
    }

    /// Insert recorded MIDI pattern at cursor position (n-offsets from lowest note)
    fn insert_midi_n_pattern(&mut self) {
        if let Some(ref pattern) = self.midi_recorded_n_pattern.clone() {
            // Insert the pattern as a quoted string
            let pattern_str = format!("\"{}\"", pattern);
            for c in pattern_str.chars() {
                self.insert_char(c);
            }
            let base_info = self.midi_recorded_base_note.as_deref().unwrap_or("?");
            self.status_message = format!(
                "📝 Inserted n-offsets: {} (base: {})",
                pattern_str, base_info
            );
        } else {
            self.status_message = "🎹 No recorded pattern (Alt+R to record)".to_string();
        }
    }

    /// Insert recorded MIDI velocity pattern at cursor position (for gain control)
    fn insert_midi_velocity_pattern(&mut self) {
        if let Some(ref pattern) = self.midi_recorded_velocity.clone() {
            // Insert the pattern as a quoted string
            let pattern_str = format!("\"{}\"", pattern);
            for c in pattern_str.chars() {
                self.insert_char(c);
            }
            self.status_message = format!("📝 Inserted velocities: {}", pattern_str);
        } else {
            self.status_message = "🎹 No recorded pattern (Alt+R to record)".to_string();
        }
    }

    /// Insert recorded MIDI legato pattern at cursor position (for articulation control)
    fn insert_midi_legato_pattern(&mut self) {
        if let Some(ref pattern) = self.midi_recorded_legato.clone() {
            // Insert the pattern as a quoted string
            let pattern_str = format!("\"{}\"", pattern);
            for c in pattern_str.chars() {
                self.insert_char(c);
            }
            self.status_message = format!("📝 Inserted legato: {}", pattern_str);
        } else {
            self.status_message = "🎹 No recorded pattern (Alt+R to record)".to_string();
        }
    }

    /// Smart paste: Insert complete pattern with auto-generated bus name (~rec1, ~rec2, etc.)
    /// Includes:
    /// - Auto-generated bus name (~rec1, ~rec2, ...)
    /// - slow N $ wrapper (if multi-cycle)
    /// - Note pattern (n "...")
    /// - Velocity pattern (# gain "...")
    /// - Legato pattern (# legato "...")
    /// - Properly aligned and formatted for readability
    fn insert_midi_smart_paste(&mut self) {
        if let Some(ref pattern) = self.midi_recorded_pattern.clone() {
            if let Some(ref velocity) = self.midi_recorded_velocity.clone() {
                if let Some(ref legato) = self.midi_recorded_legato.clone() {
                    // Increment counter and generate bus name
                    self.recording_counter += 1;
                    let rec_name = format!("~rec{}", self.recording_counter);

                    // Build slow wrapper if multi-cycle
                    let slow_wrapper = if self.midi_recorded_cycles > 1 {
                        format!("slow {} $ ", self.midi_recorded_cycles)
                    } else {
                        String::new()
                    };

                    // Build complete pattern
                    // Format: ~rec1: slow 4 $ n "c4 e4 g4" # gain "0.8 1.0 0.6" # legato "0.9 0.5 1.0"
                    let full_pattern = format!(
                        "{}: {}n \"{}\" # gain \"{}\" # legato \"{}\"",
                        rec_name, slow_wrapper, pattern, velocity, legato
                    );

                    // Insert at cursor
                    for c in full_pattern.chars() {
                        self.insert_char(c);
                    }

                    self.status_message =
                        format!("📝 Inserted {} with dynamics & legato", rec_name);
                } else {
                    self.status_message =
                        "🎹 No legato data (recording may have failed)".to_string();
                }
            } else {
                self.status_message = "🎹 No velocity data (recording may have failed)".to_string();
            }
        } else {
            self.status_message = "🎹 No recorded pattern (Alt+R to record)".to_string();
        }
    }

    /// Process incoming MIDI events (called from main loop)
    fn process_midi_events(&mut self) {
        if let Some(ref handler) = self.midi_input {
            let events = handler.recv_all();
            for event in events {
                // If recording, add to recorder
                if self.midi_recording {
                    if let Some(ref mut recorder) = self.midi_recorder {
                        recorder.record_event(event.clone());
                    }
                }

                // Show note-on events in status (feedback)
                if let MidiMessageType::NoteOn { note, velocity } = event.message_type {
                    if velocity > 0 {
                        let note_name = MidiEvent::midi_to_note_name(note);
                        self.console_messages.push(format!("🎹 {}", note_name));
                        // Keep console messages limited
                        while self.console_messages.len() > 10 {
                            self.console_messages.remove(0);
                        }
                    }
                }
            }
        }
    }

    /// Poll for VST3 parameter changes from plugin GUIs (called from main loop)
    /// Throttled to max 10Hz to prevent TUI flickering from rapid parameter updates
    #[cfg(all(target_os = "linux", feature = "vst3"))]
    fn poll_vst3_param_changes(&mut self) {
        // Throttle: only poll every 100ms to prevent TUI flickering
        if self.last_param_poll.elapsed().as_millis() < 100 {
            return;
        }
        self.last_param_poll = std::time::Instant::now();

        // Get names of open GUIs
        let gui_names: Vec<String> = self.vst3_guis.keys().cloned().collect();
        if gui_names.is_empty() {
            return;
        }

        // Collect all parameter changes first (to avoid borrow issues)
        let mut all_changes: Vec<(String, String, f64)> = Vec::new();

        // Access the signal graph to get real_plugins
        let graph_arc = self.graph.load();
        if let Some(ref graph_cell) = **graph_arc {
            if let Ok(graph) = graph_cell.0.try_borrow() {
                if let Ok(mut real_plugins) = graph.real_plugins.try_lock() {
                    for name in &gui_names {
                        if let Some(plugin) = real_plugins.get_mut(name) {
                            // Poll for parameter changes
                            if let Ok(changes) = plugin.get_param_changes() {
                                for (param_id, value) in changes {
                                    // Get parameter name from plugin info
                                    let param_name = if let Ok(info) = plugin.parameter_info(param_id as usize) {
                                        info.name
                                    } else {
                                        format!("param_{}", param_id)
                                    };

                                    all_changes.push((name.clone(), param_name, value));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Now update the phonon source text for each change
        for (name, param_name, value) in all_changes {
            // Log the change to console
            self.console_messages.push(format!(
                "🎛️ ~{} # {} {:.3}",
                name, param_name, value
            ));

            // Update the phonon code
            self.update_plugin_param_in_content(&name, &param_name, value);

            // Keep console messages limited
            while self.console_messages.len() > 10 {
                self.console_messages.remove(0);
            }
        }
    }

    /// Update a plugin parameter value in the phonon source code
    #[cfg(all(target_os = "linux", feature = "vst3"))]
    fn update_plugin_param_in_content(&mut self, instance_name: &str, param_name: &str, value: f64) {
        // Find the line containing this plugin instance (e.g., "~name $ vst ..." or "~name:")
        let pattern = format!("~{}", instance_name);
        let value_str = format!("{:.3}", value);

        let mut new_lines: Vec<String> = Vec::new();
        let mut found = false;

        for line in self.content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(&pattern) && (trimmed.contains('$') || trimmed.contains(':')) {
                found = true;
                // This is our target line - update or add the parameter
                let updated_line = self.update_param_in_line(line, param_name, &value_str);
                new_lines.push(updated_line);
            } else {
                new_lines.push(line.to_string());
            }
        }

        if found {
            self.content = new_lines.join("\n");
        }
    }

    /// Update or add a parameter in a single line
    #[cfg(all(target_os = "linux", feature = "vst3"))]
    fn update_param_in_line(&self, line: &str, param_name: &str, value: &str) -> String {
        // Check if the parameter already exists in the line
        // Pattern: # param_name followed by a value (number or expression)
        let param_pattern = format!("# {} ", param_name);

        if let Some(start) = line.find(&param_pattern) {
            // Parameter exists - find and replace the value
            let after_param = start + param_pattern.len();
            let rest = &line[after_param..];

            // Find where the value ends (at next # or end of line)
            let value_end = if let Some(next_hash) = rest.find(" #") {
                next_hash
            } else {
                rest.len()
            };

            // Reconstruct the line with the new value
            format!(
                "{}{}{}",
                &line[..after_param],
                value,
                &rest[value_end..]
            )
        } else {
            // Parameter doesn't exist - append it before any trailing comment
            // Or just append to end if no comment
            if let Some(comment_pos) = line.find("--") {
                format!(
                    "{} # {} {} {}",
                    line[..comment_pos].trim_end(),
                    param_name,
                    value,
                    &line[comment_pos..]
                )
            } else {
                format!("{} # {} {}", line.trim_end(), param_name, value)
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

    /// Ensure the cursor line is visible by adjusting scroll_offset
    fn ensure_cursor_visible(&mut self) {
        let (cursor_line, _) = self.pos_to_line_col(self.cursor_pos);
        let cursor_line = cursor_line as u16;

        // Leave some margin (2 lines) at top and bottom when possible
        let margin = 2u16;
        let visible_height = self.viewport_height.saturating_sub(4); // Account for borders

        // If cursor is above visible area, scroll up
        if cursor_line < self.scroll_offset + margin {
            self.scroll_offset = cursor_line.saturating_sub(margin);
        }

        // If cursor is below visible area, scroll down
        if cursor_line >= self.scroll_offset + visible_height.saturating_sub(margin) {
            self.scroll_offset = cursor_line.saturating_sub(visible_height.saturating_sub(margin + 1));
        }
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
            Some(t) => {
                // In Keyword context, check if there's a ':' before the token
                // This is needed because ':' is a token delimiter, so "gain :am"
                // gives us token "am" but we need to know the ':' is there
                let text = if matches!(context, completion::CompletionContext::Keyword(_)) {
                    // Check if the character before token is ':'
                    if t.start > 0 && line.chars().nth(t.start - 1) == Some(':') {
                        format!(":{}", t.text)
                    } else {
                        t.text.clone()
                    }
                } else {
                    t.text.clone()
                };
                (text, t.start)
            }
            None => {
                // No token found - check if we're in a context that allows empty completion
                match context {
                    completion::CompletionContext::Sample
                    | completion::CompletionContext::Bus
                    | completion::CompletionContext::Keyword(_)
                    | completion::CompletionContext::AfterChain
                    | completion::CompletionContext::AfterTransform
                    | completion::CompletionContext::AfterBusAssignment => {
                        // Show all completions for this context
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

        self.completion_state
            .show(completions.clone(), partial_text, line_start + token_start);

        self.status_message = format!(
            "{} completions | ↑↓: nav | Enter: accept | C-Spc: +defaults | ?: docs",
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

    /// Accept the current completion (Enter or Tab)
    fn accept_completion(&mut self) {
        self.accept_completion_inner(false);
    }

    /// Accept the current completion with default parameters (Ctrl+Space)
    fn accept_completion_with_defaults(&mut self) {
        self.accept_completion_inner(true);
    }

    /// Inner implementation for accepting completion
    fn accept_completion_inner(&mut self, with_defaults: bool) {
        if let Some(completion) = self.completion_state.accept() {
            // Replace the token at cursor with the completion
            let token_start = self.completion_state.token_start();
            let token_end = self.cursor_pos;

            if token_start <= self.content.len() {
                // Build the text to insert
                let insert_text = if with_defaults {
                    // Get kwargs template for this function
                    if let Some(metadata) = completion::FUNCTION_METADATA.get(completion.text.as_str()) {
                        let kwargs = completion::generate_kwargs_template(metadata);
                        format!("{}{}", completion.text, kwargs)
                    } else {
                        // Try generated metadata for positional defaults
                        let generated = completion::generated_metadata::get_all_functions();
                        if let Some(gen_meta) = generated.get(&completion.text) {
                            // Generate positional defaults from params
                            let defaults: Vec<String> = gen_meta.params.iter()
                                .map(|p| p.default.clone().unwrap_or_else(|| "_".to_string()))
                                .collect();
                            if defaults.is_empty() {
                                completion.text.clone()
                            } else {
                                format!("{} {}", completion.text, defaults.join(" "))
                            }
                        } else {
                            completion.text.clone()
                        }
                    }
                } else {
                    completion.text.clone()
                };

                self.content
                    .replace_range(token_start..token_end, &insert_text);
                self.cursor_pos = token_start + insert_text.len();

                if with_defaults {
                    self.status_message = format!("✓ {} (with defaults)", completion.text);
                } else {
                    self.status_message = format!("✓ {}", completion.text);
                }
            }
        }
    }

    /// Cancel completion (Esc or movement)
    fn cancel_completion(&mut self) {
        self.completion_state.hide();
    }

    /// Expand function with all kwargs and default values (Shift+Tab)
    ///
    /// Takes a function name like "gain" and expands it to "gain :amount 1.0"
    /// or "plate" to "plate :pre_delay 0.02 :decay 0.7 :diffusion 0.7 :damping 0.3 :mod_depth 0.3 :mix 0.5"
    fn expand_kwargs_template(&mut self) {
        // Get current line and cursor position
        let (line_idx, col) = self.pos_to_line_col(self.cursor_pos);
        let lines: Vec<&str> = self.content.split('\n').collect();

        if line_idx >= lines.len() {
            return;
        }

        let line = lines[line_idx];

        // Find the function name at cursor
        if let Some(func_name) = self.find_function_at_cursor(line, col) {
            // Get parameter metadata for this function
            if let Some(metadata) = completion::FUNCTION_METADATA.get(func_name) {
                // Generate kwargs template
                let template = completion::generate_kwargs_template(metadata);

                if !template.is_empty() {
                    // Insert template at cursor
                    self.push_undo();
                    self.content.insert_str(self.cursor_pos, &template);
                    self.cursor_pos += template.len();
                    self.status_message = format!("✓ Expanded {} with kwargs", func_name);
                }
            } else {
                self.status_message = format!("No metadata for function: {}", func_name);
            }
        } else {
            self.status_message = "No function found at cursor".to_string();
        }
    }

    /// Find function name at cursor position
    ///
    /// Looks backwards from cursor to find the last valid function name.
    /// Similar to detect_keyword_context but simpler - just finds the function.
    fn find_function_at_cursor(&self, line: &str, cursor_pos: usize) -> Option<&'static str> {
        if cursor_pos > line.len() {
            return None;
        }

        // Get text before cursor
        let before_cursor = &line[..cursor_pos];

        // Split by delimiters and find last valid token
        let tokens: Vec<&str> = before_cursor
            .split(|c: char| c.is_whitespace() || "(){}[]#$:\"".contains(c))
            .collect();

        // Get last non-empty token that's in FUNCTION_METADATA
        for token in tokens.iter().rev() {
            if token.is_empty() {
                continue;
            }

            // Check if this is a known function
            if completion::FUNCTION_METADATA.contains_key(token) {
                return completion::FUNCTION_METADATA
                    .get(token)
                    .map(|meta| meta.name);
            }
        }

        None
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
            Some(t) => {
                // In Keyword context, check if there's a ':' before the token
                // This is needed because ':' is a token delimiter, so "gain :am"
                // gives us token "am" but we need to know the ':' is there
                let text = if matches!(context, completion::CompletionContext::Keyword(_)) {
                    // Check if the character before token is ':'
                    if t.start > 0 && line.chars().nth(t.start - 1) == Some(':') {
                        format!(":{}", t.text)
                    } else {
                        t.text.clone()
                    }
                } else {
                    t.text.clone()
                };
                (text, t.start)
            }
            None => {
                // No token - check if we're in a completable context
                match context {
                    completion::CompletionContext::Sample
                    | completion::CompletionContext::Bus
                    | completion::CompletionContext::Keyword(_) => {
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

        self.completion_state
            .show(completions.clone(), partial_text, line_start + token_start);

        self.status_message = format!(
            "{} completions | ↑↓: nav | Enter: accept | C-Spc: +defaults | ?: docs",
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

    /// Handle key events when plugin browser is visible
    fn handle_plugin_browser_key_event(&mut self, key: KeyEvent) -> KeyResult {
        // Handle naming mode specially
        if self.plugin_browser.is_naming() {
            match key.code {
                KeyCode::Esc => {
                    self.plugin_browser.cancel_naming();
                    KeyResult::Continue
                }
                KeyCode::Enter => {
                    if let Some(instance_name) = self.plugin_browser.confirm_naming() {
                        // Get the selected plugin and create the instance
                        if let Some(plugin) = self.plugin_browser.selected_plugin(&self.plugin_manager) {
                            let plugin_name = plugin.id.name.clone();
                            match self.plugin_manager.create_named_instance(&plugin_name, &instance_name) {
                                Ok(()) => {
                                    // Insert instance reference at cursor
                                    let insert_text = format!("~{}", instance_name);
                                    self.insert_text(&insert_text);
                                    self.plugin_browser.hide();
                                }
                                Err(e) => {
                                    self.plugin_browser.set_status(format!("Error: {}", e));
                                }
                            }
                        }
                    }
                    KeyResult::Continue
                }
                KeyCode::Char(c) => {
                    self.plugin_browser.add_char(c);
                    KeyResult::Continue
                }
                KeyCode::Backspace => {
                    self.plugin_browser.delete_char();
                    KeyResult::Continue
                }
                _ => KeyResult::Continue,
            }
        } else {
            match key.code {
                // Esc or Alt+P: Close browser
                KeyCode::Esc => {
                    self.plugin_browser.hide();
                    KeyResult::Continue
                }
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::ALT) => {
                    self.plugin_browser.toggle();
                    KeyResult::Continue
                }

                // Tab: Switch between Available/Instances views
                KeyCode::Tab => {
                    self.plugin_browser.toggle_view();
                    KeyResult::Continue
                }

                // Alt+G: Open GUI for loaded plugins (from audio graph)
                #[cfg(all(target_os = "linux", feature = "vst3"))]
                KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::ALT) => {
                    self.open_plugin_guis();
                    KeyResult::Continue
                }

                // Ctrl+G: Open GUI preview for selected plugin (loads plugin if needed)
                #[cfg(all(target_os = "linux", feature = "vst3"))]
                KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.open_preview_gui();
                    KeyResult::Continue
                }

                // Navigation
                KeyCode::Up => {
                    self.plugin_browser.select_prev();
                    KeyResult::Continue
                }
                KeyCode::Down => {
                    let max_items = match self.plugin_browser.current_view() {
                        plugin_browser::BrowserView::Available => {
                            self.plugin_manager.list_plugins().len()
                        }
                        plugin_browser::BrowserView::Instances => {
                            self.plugin_manager.list_instances().len()
                        }
                    };
                    self.plugin_browser.select_next(max_items);
                    KeyResult::Continue
                }

                // Enter: Insert vst code (available view) or insert reference (instances view)
                KeyCode::Enter => {
                    match self.plugin_browser.current_view() {
                        plugin_browser::BrowserView::Available => {
                            if let Some(plugin) = self.plugin_browser.selected_plugin(&self.plugin_manager) {
                                // Insert vst "PluginName" code directly
                                let insert_text = format!("vst \"{}\"", plugin.id.name);
                                self.insert_text(&insert_text);
                                self.plugin_browser.hide();
                            }
                        }
                        plugin_browser::BrowserView::Instances => {
                            if let Some(name) = self.plugin_browser.selected_instance_name(&self.plugin_manager) {
                                // Insert instance reference at cursor
                                let insert_text = format!("~{}", name);
                                self.insert_text(&insert_text);
                                self.plugin_browser.hide();
                            }
                        }
                    }
                    KeyResult::Continue
                }

                // Character input for filter
                KeyCode::Char(c) => {
                    self.plugin_browser.add_char(c);
                    KeyResult::Continue
                }

                // Backspace for filter
                KeyCode::Backspace => {
                    self.plugin_browser.delete_char();
                    KeyResult::Continue
                }

                _ => KeyResult::Continue,
            }
        }
    }

    /// Find VST plugin name under cursor (looks for `vst "PluginName"` on current line)
    fn get_vst_under_cursor(&self) -> Option<String> {
        // Find current line
        let mut current_pos = 0;
        for line in self.content.lines() {
            let line_end = current_pos + line.len();
            if current_pos <= self.cursor_pos && self.cursor_pos <= line_end {
                // Found the line containing cursor
                // Look for vst "PluginName" pattern
                if let Some(vst_idx) = line.find("vst \"") {
                    let start = vst_idx + 5; // after 'vst "'
                    if let Some(end_quote) = line[start..].find('"') {
                        let plugin_name = &line[start..start + end_quote];
                        return Some(plugin_name.to_string());
                    }
                }
                return None;
            }
            current_pos = line_end + 1; // +1 for newline
        }
        None
    }

    /// Open VST3 GUIs - if cursor is on a vst line, open just that one
    /// Only available on Linux with vst3 feature
    #[cfg(all(target_os = "linux", feature = "vst3"))]
    /// Auto-configure XWayland environment for GNOME Wayland sessions
    /// This finds DISPLAY from systemd and the mutter Xauthority file
    fn setup_xwayland_env() -> Result<(), String> {
        // If DISPLAY is not set, try to get it from systemd user environment
        if std::env::var("DISPLAY").is_err() {
            if let Ok(output) = std::process::Command::new("systemctl")
                .args(["--user", "show-environment"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Some(display) = line.strip_prefix("DISPLAY=") {
                        std::env::set_var("DISPLAY", display);
                        break;
                    }
                }
            }
        }

        // If XAUTHORITY is not set, find mutter's Xwaylandauth file
        if std::env::var("XAUTHORITY").is_err() {
            let uid = unsafe { libc::getuid() };
            let auth_dir = format!("/run/user/{}", uid);
            if let Ok(entries) = std::fs::read_dir(&auth_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with(".mutter-Xwaylandauth.") {
                        std::env::set_var("XAUTHORITY", entry.path());
                        break;
                    }
                }
            }
        }

        // Verify we have what we need
        if std::env::var("DISPLAY").is_err() {
            return Err("Could not find DISPLAY (not in a graphical session?)".to_string());
        }

        Ok(())
    }

    fn open_plugin_guis(&mut self) {
        // Debug helper - write to file since stderr is redirected
        fn log_gui(msg: &str) {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/phonon_gui_debug.log")
            {
                let _ = writeln!(f, "{}", msg);
            }
        }

        // Auto-configure XWayland environment (for GNOME Wayland)
        if let Err(e) = Self::setup_xwayland_env() {
            log_gui(&format!("XWayland setup failed: {}", e));
            self.status_message = e.clone();
            self.plugin_browser.set_status(&e);
            return;
        }

        // Debug: Log DISPLAY value
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| "NOT SET".to_string());
        let xauth = std::env::var("XAUTHORITY").unwrap_or_else(|_| "NOT SET".to_string());
        log_gui(&format!("=== Alt+G pressed, DISPLAY={}, XAUTHORITY={} ===", display, xauth));
        self.add_console_message(&format!("🔍 GUI: DISPLAY={}", display));

        // Check if cursor is on a VST line
        let target_plugin = self.get_vst_under_cursor();
        log_gui(&format!("Target plugin under cursor: {:?}", target_plugin));

        // Get the current graph
        let graph_guard = self.graph.load();
        let graph_opt = graph_guard.as_ref();

        if let Some(graph_cell) = graph_opt {
            log_gui("Graph is loaded");

            // Borrow the graph structure (still RefCell)
            let graph = match graph_cell.0.try_borrow() {
                Ok(g) => g,
                Err(_) => {
                    let msg = "Graph busy - try again";
                    log_gui(msg);
                    self.status_message = msg.to_string();
                    self.plugin_browser.set_status(msg);
                    return;
                }
            };

            // Lock real_plugins (Mutex - will block until available)
            // This properly waits for the audio thread to release the lock
            let mut real_plugins = graph.real_plugins.lock().unwrap();
            log_gui(&format!("Loaded plugins: {:?}", real_plugins.keys().collect::<Vec<_>>()));

            if real_plugins.is_empty() {
                let msg = "No VST3 plugins loaded. Use 'vst \"plugin_name\"' in your code.";
                log_gui(msg);
                self.status_message = msg.to_string();
                self.plugin_browser.set_status(msg);
                return;
            }

            // If we have a target plugin from cursor, check if it's loaded
            if let Some(ref target) = target_plugin {
                if !real_plugins.contains_key(target) {
                    let msg = format!("Plugin '{}' not loaded. Press C-x to evaluate first.", target);
                    self.status_message = msg.clone();
                    self.plugin_browser.set_status(&msg);
                    return;
                }
            }

            let mut opened_count = 0;
            let mut errors = Vec::new();

            // Iterate through loaded plugins
            for (name, plugin) in real_plugins.iter_mut() {
                // If we have a target, skip non-matching plugins
                if let Some(ref target) = target_plugin {
                    if name != target {
                        continue;
                    }
                }

                // Skip if we already have a GUI for this plugin
                if self.vst3_guis.contains_key(name) {
                    // If targeting specific plugin, report it's already open
                    if target_plugin.is_some() {
                        let msg = format!("🎛️ {} GUI already open", name);
                        self.status_message = msg.clone();
                        self.plugin_browser.set_status(&msg);
                        return;
                    }
                    continue;
                }

                // Try to create GUI
                log_gui(&format!("Creating GUI for: {}", name));
                match plugin.create_gui() {
                    Ok(mut gui) => {
                        log_gui(&format!("GUI created for {}, calling show()...", name));
                        // Show the GUI window
                        if let Err(e) = gui.show(Some(name)) {
                            log_gui(&format!("show() FAILED for {}: {}", name, e));
                            errors.push(format!("{}: show failed - {}", name, e));
                            continue;
                        }
                        log_gui(&format!("GUI show() succeeded for {}", name));

                        // Store the GUI handle
                        self.vst3_guis.insert(name.clone(), gui);
                        opened_count += 1;
                    }
                    Err(e) => {
                        log_gui(&format!("create_gui() FAILED for {}: {}", name, e));
                        // Provide helpful hint about X11 authorization issues
                        let err_str = e.to_string();
                        if err_str.contains("display") || err_str.contains("unavailable") {
                            errors.push(format!("{}: X11 error (try: xhost +local:)", name));
                        } else {
                            errors.push(format!("{}: {}", name, e));
                        }
                    }
                }
            }

            // Set status message (both main status and plugin browser)
            let msg = if opened_count > 0 {
                if let Some(ref target) = target_plugin {
                    format!("🎛️ Opened {} GUI", target)
                } else {
                    format!("🎛️ Opened {} plugin GUI(s)", opened_count)
                }
            } else if !errors.is_empty() {
                format!("GUI errors: {}", errors.join(", "))
            } else {
                "All plugin GUIs already open".to_string()
            };
            self.status_message = msg.clone();
            self.plugin_browser.set_status(&msg);
        } else {
            let msg = "No audio graph loaded. Press C-x to evaluate code first.";
            self.status_message = msg.to_string();
            self.plugin_browser.set_status(msg);
        }
    }

    /// Insert text at current cursor position
    fn insert_text(&mut self, text: &str) {
        // Push undo state
        self.push_undo();

        // Insert text at cursor
        let (before, after) = self.content.split_at(self.cursor_pos);
        self.content = format!("{}{}{}", before, text, after);
        self.cursor_pos += text.len();
    }

    /// Open GUI preview for selected plugin in browser (loads plugin if needed)
    #[cfg(all(target_os = "linux", feature = "vst3"))]
    fn open_preview_gui(&mut self) {
        use crate::plugin_host::real_plugin::create_real_plugin_by_name;

        // Setup XWayland environment
        if let Err(e) = Self::setup_xwayland_env() {
            self.plugin_browser.set_status(&e);
            return;
        }

        // Get selected plugin from browser
        let plugin_info = match self.plugin_browser.selected_plugin(&self.plugin_manager) {
            Some(p) => p.clone(),
            None => {
                self.plugin_browser.set_status("No plugin selected");
                return;
            }
        };

        let plugin_name = plugin_info.id.name.clone();

        // Check if GUI is already open
        if self.vst3_guis.contains_key(&plugin_name) {
            self.plugin_browser.set_status(&format!("{} GUI already open", plugin_name));
            return;
        }

        // Check if we already have this plugin loaded (preview or audio graph)
        let has_preview = self.preview_plugins.contains_key(&plugin_name);

        if !has_preview {
            // Load the plugin for preview
            self.plugin_browser.set_status(&format!("Loading {}...", plugin_name));
            match create_real_plugin_by_name(&plugin_name) {
                Ok(mut plugin) => {
                    // Initialize plugin
                    if let Err(e) = plugin.initialize(48000.0, 512) {
                        self.plugin_browser.set_status(&format!("Init failed: {}", e));
                        return;
                    }
                    self.preview_plugins.insert(plugin_name.clone(), plugin);
                }
                Err(e) => {
                    self.plugin_browser.set_status(&format!("Load failed: {}", e));
                    return;
                }
            }
        }

        // Now open the GUI
        if let Some(plugin) = self.preview_plugins.get_mut(&plugin_name) {
            match plugin.create_gui() {
                Ok(mut gui) => {
                    if let Err(e) = gui.show(Some(&plugin_name)) {
                        self.plugin_browser.set_status(&format!("Show failed: {}", e));
                        return;
                    }
                    self.vst3_guis.insert(plugin_name.clone(), gui);
                    self.plugin_browser.set_status(&format!("Opened {} GUI", plugin_name));
                }
                Err(e) => {
                    self.plugin_browser.set_status(&format!("GUI failed: {}", e));
                }
            }
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
