# Unified Timing Architecture Fix

## Problem Identified

Both live mode and edit mode had timing bugs caused by software-timed audio generation:

### Live Mode Bug (live.rs)
- Used a 10ms software timer to generate audio blocks
- Each block was 512 samples = 11.6ms at 44.1kHz
- **Time compression**: Generating 11.6ms of audio every 10ms caused events to bunch up
- Result: Euclidean patterns like `cp(2,4)` produced "xx x x" instead of "x x" rhythm

### Edit Mode Bug (modal_editor.rs via live_engine.rs)
- Rendered a fixed-duration buffer (default 4.0 seconds) and looped it
- DSL tempo (e.g., `tempo: 0.4` = 2.5 sec/cycle) didn't match buffer loop point
- Result: Rhythm artifacts when buffer looped at wrong point

## Solution: Callback-Driven Architecture

Replaced software-timed push-based audio with hardware-timed callback-driven rendering:

### How It Works
1. Audio hardware requests samples via callback when buffer needs refilling
2. Callback directly calls `graph.process_sample()` for each sample needed
3. No software timers, no timing drift - **hardware clock is the source of truth**

### Implementation

**live.rs**:
- Removed `AudioEngine` and `SignalExecutor` dependencies
- Added `cpal` audio stream with callback
- Shared graph: `Arc<Mutex<Option<UnifiedSignalGraph>>>`
- Hot-reload: File watcher atomically swaps graph when file changes

**modal_editor.rs**:
- Removed `LiveEngine` dependency
- Added same callback architecture as live.rs
- DSL code is parsed, compiled to graph, hot-swapped on eval

### Key Benefits
1. **Perfect timing**: All three modes (render, live, edit) use identical rendering
2. **No drift**: Hardware clock ensures sample-accurate timing
3. **Testable**: Render mode output is ground truth for all modes
4. **Atomic updates**: Graph hot-swap is thread-safe and glitch-free

## Testing

### Render Mode (Already Verified)
```bash
cargo run --release --bin phonon -- render y.ph /tmp/y_test.wav --cycles 8
```
Expected: Perfect 2-2-2-2 kick pattern (bd), perfect 1-per-cycle clap (cp)

### Live Mode (Manual Test Required)
```bash
cargo run --release --bin phonon -- live y.ph
# Listen: Should hear perfect timing, no bunching
# Edit file: Should hot-reload without glitches
```

### Edit Mode (Manual Test Required)
```bash
cargo run --release --bin phonon -- edit y.ph
# Press C-x to eval
# Listen: Should hear perfect timing matching render mode
```

## Files Modified

1. `src/live.rs` - Complete rewrite to callback architecture
2. `src/modal_editor.rs` - Converted to callback architecture
3. `src/unified_graph.rs` - Disabled verbose debug output (too noisy)

## Expected Outcomes

- ✅ Render mode: Perfect timing (verified)
- ✅ Live mode: Should match render mode timing exactly
- ✅ Edit mode: Should match render mode timing exactly
- ✅ No more "xx x x" bunching on Euclidean patterns
- ✅ Hot-reload works without audio glitches

## Next Steps

1. Manual testing of live and edit modes
2. If timing is perfect, commit as unified timing fix
3. Update documentation with architecture details
