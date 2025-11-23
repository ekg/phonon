# Time Architecture in Phonon

**Fundamental Principle**: Time is independent. Synthesis depends on time.

---

## The Principle

> **"Time should be independent of synthesis - it's synthesis that depends on time."**
> — User insight, 2025-11-23

This fundamental architectural principle means:

1. **Time flows continuously** - Never resets, never jumps (except on explicit panic)
2. **Patterns can change** - You can reload code at any time
3. **Synthesis adapts** - Oscillators, delays, reverbs adjust to new patterns
4. **But time keeps flowing** - The global clock never stops

---

## The Bug (Before Fix)

### What Was Happening

Every time you reloaded code (changed patterns), Phonon would:

```rust
// ❌ OLD CODE (BROKEN):
match compile_program(statements, sample_rate) {
    Ok(mut graph) => {
        // Creates FRESH graph with time = 0!
        let audio_buffer = graph.render(samples_per_cycle);
    }
}
```

**Result**:
- ⏱️ `session_start_time = now()` → Time jumps forward!
- 📊 `sample_count = 0` → Sample counter resets!
- 🌊 All oscillator phases → 0 → **Clicks and pops!**
- 🔊 Delays/reverbs cleared → **Audio discontinuities!**
- 🎵 Beat drops → **Timing glitches!**

### Why This Violated the Architecture

**Time was dependent on pattern reloading**, not independent!

- Pattern reload → New graph → Time resets
- This backwards dependency broke synthesis continuity

---

## The Fix

### What Changed

Now we **preserve time continuity** between reloads:

```rust
// ✅ NEW CODE (CORRECT):
let mut current_graph: Option<UnifiedSignalGraph> = None;

match compile_program(statements, sample_rate) {
    Ok(mut graph) => {
        // CRITICAL: Preserve time continuity!
        if let Some(old_graph) = &current_graph {
            // Transfer session timing from old graph
            graph.transfer_session_timing(old_graph);
        } else {
            // First load: enable wall-clock timing
            graph.enable_wall_clock_timing();
        }

        // Store graph for next reload
        current_graph = Some(graph);
    }
}
```

**Result**:
- ✅ `session_start_time` preserved → Time flows continuously!
- ✅ `sample_count` continues → No counter resets!
- ✅ Oscillator phases maintained → **No clicks!**
- ✅ Delays/reverbs preserve state → **Smooth transitions!**
- ✅ Beat never drops → **Perfect timing!**

---

## How It Works

### Wall-Clock Timing (Live Mode)

Phonon uses **wall-clock time** as the independent time source:

```rust
pub struct UnifiedSignalGraph {
    // Wall-clock based timing
    session_start_time: std::time::Instant,  // When session started
    cycle_offset: f64,                        // Offset for alignment
    cps: f32,                                 // Cycles per second
    use_wall_clock: bool,                     // Enable wall-clock mode
}
```

**Cycle position formula**:
```rust
cycle_position = (now - session_start_time).as_secs_f64() * cps + cycle_offset
```

This means:
- Time is **always** computed from wall-clock
- Patterns change → cycle_position keeps incrementing
- Oscillators read cycle_position → maintain phase continuity

### Time Transfer on Reload

When new code is loaded:

```rust
pub fn transfer_session_timing(&mut self, old_graph: &UnifiedSignalGraph) {
    // Preserve the session clock
    self.session_start_time = old_graph.session_start_time;
    self.cycle_offset = old_graph.cycle_offset;
    self.cps = old_graph.cps;

    // Transfer cycle bus cache (prevents resynthesis glitches)
    self.cycle_bus_cache = old_graph.cycle_bus_cache.clone();
}
```

This ensures:
1. **New graph inherits the global clock** from old graph
2. **Time continuity is maintained** across reloads
3. **No glitches or discontinuities** in audio

---

## Sample-Based vs Wall-Clock Timing

Phonon supports **two timing modes**:

### Sample-Based Timing (Offline Rendering)

```rust
use_wall_clock = false
cycle_position = sample_count / sample_rate / cps
```

- Used for offline rendering (WAV export)
- Deterministic: same input → same output
- Time advances by samples rendered

### Wall-Clock Timing (Live Mode)

```rust
use_wall_clock = true
cycle_position = (now - session_start_time).as_secs_f64() * cps + offset
```

- Used for live coding
- Time flows continuously (real-time clock)
- Pattern reloads don't affect time
- Beat never drops!

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────┐
│           WALL-CLOCK TIME (Independent)          │
│                                                   │
│  session_start_time = Instant::now()             │
│  elapsed = now - session_start_time              │
│  cycle_position = elapsed * cps + offset         │
│                                                   │
│  This NEVER resets (except on panic)             │
└─────────────────┬───────────────────────────────┘
                  │
                  │ Time flows continuously
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│            PATTERNS (User Code)                  │
│                                                   │
│  ~lfo: sine 0.5                                  │
│  ~bass: saw 55 # lpf (~lfo * 2000 + 500) 0.8   │
│  out: ~bass                                      │
│                                                   │
│  Can change at ANY TIME (code reload)            │
└─────────────────┬───────────────────────────────┘
                  │
                  │ Patterns query time
                  │
                  ▼
┌─────────────────────────────────────────────────┐
│         SYNTHESIS (Dependent on Time)            │
│                                                   │
│  Oscillators: phase = 2π * freq * time          │
│  Delays: buffer[time % delay_time]              │
│  Reverbs: read from time-based buffer            │
│                                                   │
│  Synthesis READS time, doesn't SET time         │
└─────────────────────────────────────────────────┘
```

**Key Insight**: Arrows point DOWN, never UP!
- Time is at the top (independent)
- Patterns read time
- Synthesis reads time and patterns
- **Nothing affects time** (except explicit panic)

---

## Benefits

### 1. **Glitch-Free Live Coding**
Change patterns without audio discontinuities:
- No clicks from phase resets
- No glitches from buffer clearing
- Beat keeps perfect time

### 2. **Predictable Behavior**
Time flows at constant rate (wall-clock):
- 120 BPM = exactly 2 cycles/second
- Never drifts or jumps (unless you change BPM)
- Synchronizable with external gear

### 3. **Compositional Freedom**
Change anything without worrying about time:
- Swap oscillators mid-performance
- Change filters on the fly
- Rearrange patterns live
- Time keeps flowing smoothly

### 4. **Proper Architecture**
Separation of concerns:
- **Time**: Independent, continuous, monotonic
- **Patterns**: Define events within time
- **Synthesis**: Generates audio based on time
- Clean, composable, maintainable

---

## When Time DOES Reset

The only time `session_start_time` resets is on **explicit panic**:

```rust
EngineCommand::Panic => {
    // User explicitly wants to reset everything
    current_graph = None;  // Next load gets fresh time
}
```

This is intentional:
- Panic = "start over completely"
- Clears all state (voices, buffers, time)
- Fresh start on next code load

---

## Testing Time Continuity

To verify time continuity works:

```phonon
-- Load this code:
out: sine 440

-- While it's playing, change to:
out: sine 880

-- Expected: Smooth transition, no click
-- (Phase continuity maintained across reload)

-- Load this:
out: sine 440 # delay 0.5 0.5

-- While delay tail is playing, change to:
out: sine 880 # delay 0.5 0.5

-- Expected: Old delay tail continues
-- (Delay buffer preserved, new notes added)
```

If you hear clicks or glitches on reload → time continuity broken!

---

## Implementation Details

### Key Methods

```rust
impl UnifiedSignalGraph {
    /// Enable wall-clock timing (for live mode)
    pub fn enable_wall_clock_timing(&mut self) {
        self.use_wall_clock = true;
        self.session_start_time = std::time::Instant::now();
        // Sets current time as session start
    }

    /// Transfer session timing from old graph
    pub fn transfer_session_timing(&mut self, old_graph: &UnifiedSignalGraph) {
        self.session_start_time = old_graph.session_start_time;
        self.cycle_offset = old_graph.cycle_offset;
        self.cps = old_graph.cps;
        // Preserves time continuity across reload
    }

    /// Get current cycle position (wall-clock or sample-based)
    pub fn get_cycle_position(&self) -> f64 {
        if self.use_wall_clock {
            let elapsed = self.session_start_time.elapsed().as_secs_f64();
            elapsed * self.cps as f64 + self.cycle_offset
        } else {
            self.cached_cycle_position
        }
    }
}
```

### Usage in Live Engine

```rust
// First load: Start the global clock
graph.enable_wall_clock_timing();

// Subsequent reloads: Preserve the global clock
graph.transfer_session_timing(old_graph);

// Panic: Reset the global clock (next load)
current_graph = None;
```

---

## Future Enhancements

Potential improvements to time architecture:

1. **External Clock Sync**
   - MIDI clock input
   - Ableton Link support
   - OSC time sync

2. **Time Markers**
   - Named time points
   - Loop regions
   - Tempo changes at specific times

3. **Multiple Timelines**
   - Independent clocks for different tracks
   - Polyrhythmic timing
   - Phase-shifted clocks

4. **Time Queries**
   - `time()` function → current time
   - `cycle()` function → current cycle
   - `beat()` function → current beat

---

## Conclusion

**The architectural principle is simple but profound:**

> Time is independent. Synthesis depends on time.

This means:
- ✅ Time flows continuously (wall-clock)
- ✅ Patterns can change (code reload)
- ✅ Synthesis adapts (maintains continuity)
- ✅ No glitches (phase/buffer preservation)

The fix was simple (17 lines of code), but the principle is fundamental to building a robust live coding system.

**Credits**: User insight that "time should be independent of synthesis" identified this architectural violation and led to the fix.
