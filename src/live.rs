//! Live coding module for Phonon
//!
//! Historically this module hosted `LiveSession`/`MultiFileWatcher`, a
//! file-watching hot-reload engine that swapped the active graph through a
//! `RefCell<UnifiedSignalGraph>` using panicking `.borrow()`/`.borrow_mut()`.
//! That path was the last raw-borrow graph swap in the tree (rt-safety F-10 /
//! C4) and was unreachable from every CLI command — the real `phonon live`
//! command carries its own ring-buffer synthesis loop in `main.rs`, and the
//! render-owner primitive (`src/render_swap.rs`) is now the single blessed
//! swap channel. `LiveSession`/`MultiFileWatcher` were therefore retired
//! rather than migrated, since they had no consumers.
//!
//! Only the `LiveRepl` stub remains — it is still referenced by the `phonon
//! repl` command but performs no rendering (and holds no borrow), so it keeps
//! no raw-borrow swap path alive.

/// Simple REPL for live DSL evaluation.
///
/// Currently disabled: `run` prints a notice and returns an error directing the
/// user to `phonon live file.ph` for accurate playback. Retained only so the
/// `phonon repl` CLI command continues to build.
pub struct LiveRepl {}

impl LiveRepl {
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }

    pub fn run(&mut self) -> Result<(), String> {
        println!("🎵 Phonon Live REPL");
        println!("==================");
        println!("⚠️  Warning: REPL mode may have timing issues");
        println!("   Use 'phonon live file.ph' for accurate playback");
        println!("\nType 'exit' to quit\n");

        Err("REPL mode temporarily disabled - use 'phonon live file.ph' instead".to_string())
    }
}
