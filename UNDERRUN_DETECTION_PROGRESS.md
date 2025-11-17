# Underrun Detection - Implementation Complete ✅

**Status**: Underrun detection feedback now visible in UI

## Problem Statement

User reported: "m.ph is ragged. not sure if underrunning or just garbage synthesis. there isn't any underrun feedback in the phonon edit interface."

Previously:
- Underrun detection existed but only logged to stderr
- Stderr redirected to `/tmp/phonon_audio_errors.log` for TUI cleanup
- User couldn't see if underruns were happening

## Solution Implemented

Added atomic underrun counter visible in the UI:

### Changes Made (`src/modal_editor/mod.rs`)

1. **Added atomic counter** (line 39-40, 89):
   ```rust
   use std::sync::atomic::{AtomicUsize, Ordering};

   /// Underrun counter (shared with audio callback)
   underrun_count: Arc<AtomicUsize>,
   ```

2. **Initialized in new()** (line 140):
   ```rust
   let underrun_count = Arc::new(AtomicUsize::new(0));
   ```

3. **Cloned into audio callbacks** (lines 201-202):
   ```rust
   let underrun_count_f32 = Arc::clone(&underrun_count);
   let underrun_count_i16 = Arc::clone(&underrun_count);
   ```

4. **Increment on underrun** (F32: line 223, I16: line 256):
   ```rust
   underrun_count_f32.fetch_add(1, Ordering::Relaxed);
   ```

5. **Display in UI status bar** (lines 760-783):
   ```rust
   let underrun_count = self.underrun_count.load(Ordering::Relaxed);
   let has_underruns = underrun_count > 0;

   // Red status bar when underruns detected
   let status_style = if self.error_message.is_some() || has_underruns {
       Style::default().fg(Color::Red)
   } else { ... }

   // Show underrun count prominently
   let status_text = if has_underruns {
       format!("⚠️  Audio underruns: {} (synthesis too slow!)", underrun_count)
   } else { ... };
   ```

## Testing Results

### Render Mode (Non-realtime)

Tested `m.ph` with `stut 8` over 8 cycles:
```bash
cargo run --release --bin phonon -- render m.ph /tmp/m_test.wav --cycles 8
```

**Result**: ✅ **SUCCESS**
- Completed cleanly in 16 seconds (8 cycles @ 0.5 CPS)
- RMS level: 0.174 (-15.2 dB)
- Peak level: 1.000 (normalized)
- File size: 1.4 MB
- No crashes, clean output

This confirms the **voice accumulation fix is working** (from previous session).

### Live Mode Testing

**Next step**: User should test `phonon edit m.ph` and observe:
1. Does the status bar show underruns?
2. If no underruns, but still "ragged" → synthesis quality issue
3. If yes underruns → need to optimize synthesis performance

## What This Reveals

The underrun counter will tell us definitively:
- **Underruns > 0**: Synthesis thread can't keep up with audio callback
  - Ring buffer runs dry
  - Causes clicks, dropouts, "ragged" sound
  - Fix: Optimize synthesis, larger ring buffer, or reduce pattern complexity

- **Underruns = 0**: Audio is smooth from buffering perspective
  - "Ragged" sound must be synthesis quality issue
  - Could be: incorrect envelopes, timing glitches, voice management bugs
  - Fix: Debug synthesis logic

## Current Architecture (Single Process, Threaded)

**Thread 1: Main/UI**
- Handles keyboard input, rendering TUI
- Compiles DSL code on Ctrl-X
- Atomically swaps graph via ArcSwap

**Thread 2: Background Synthesis**
- Continuously renders 512-sample chunks
- Writes to 2-second ring buffer
- Reads graph via ArcSwap (lock-free)

**Thread 3: Audio Callback (cpal managed)**
- Just reads from ring buffer (FAST!)
- Increments underrun counter if buffer empty
- Runs in real-time audio thread (high priority)

**Why this works**:
- Compilation never blocks audio (separate threads)
- Ring buffer smooths synthesis spikes (2 seconds = huge cushion)
- ArcSwap enables instant atomic graph swaps
- State transfer preserves timing and active voices

## Two-Process Architecture (REJECTED as Overkill)

Earlier exploration: Separate pattern engine and audio engine processes communicating via Unix sockets.

**Why rejected**:
- User: "yes the threaded arch is enough. we don't need two completely different processes and expensive and higher latency IPC"
- Current architecture already provides instant swapping
- IPC adds latency without benefit
- More complex for eventual web browser target

**Artifacts created** (now obsolete but functional):
- `src/ipc.rs` - Unix socket IPC protocol
- `src/bin/phonon-audio.rs` - Standalone audio engine
- `src/bin/test_two_process.rs` - Test harness (works!)
- `TWO_PROCESS_ARCHITECTURE_PROGRESS.md` - Documentation

These could be useful for future distributed architecture, but not needed now.

## Next Steps

1. **User testing**: Run `phonon edit m.ph` and observe underrun count
2. **If underruns detected**:
   - Profile synthesis performance
   - Optimize hot paths
   - Consider larger ring buffer
3. **If no underruns**:
   - Audio analysis to characterize "ragged" quality
   - Debug synthesis envelopes, timing, voice management
4. **Compare** m.ph in live mode vs rendered output quality

## Files Modified

- `src/modal_editor/mod.rs` - Added atomic underrun counter and UI display

## Files Created (Two-Process Exploration - Optional)

- `src/ipc.rs`
- `src/bin/phonon-audio.rs`
- `src/bin/test_two_process.rs`
- `TWO_PROCESS_ARCHITECTURE_PROGRESS.md`
- `UNDERRUN_DETECTION_PROGRESS.md` (this file)

## Commit Message

```
Add underrun detection feedback to phonon edit UI

Problem:
- User reported "ragged" audio in live mode
- Couldn't tell if underruns or synthesis quality issue
- Underrun detection existed but only logged to stderr

Solution:
- Add Arc<AtomicUsize> underrun counter shared with audio callback
- Display prominently in status bar (red when underruns detected)
- Shows exact count: "⚠️ Audio underruns: N (synthesis too slow!)"

This enables debugging whether "ragged" audio is from:
1. Underruns (synthesis too slow) → optimize performance
2. No underruns (synthesis quality) → debug synthesis logic

Testing:
- m.ph renders cleanly in render mode (stut 8 works!)
- Live mode testing needed to observe underrun behavior

Files:
- src/modal_editor/mod.rs: Add atomic counter, UI display
```
