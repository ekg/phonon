# Global Clock Architecture for Phonon

## Problem

Current implementation (src/unified_graph.rs:3894):
```rust
self.cycle_position += self.cps as f64 / self.sample_rate as f64;
```

**Issues:**
- ❌ Sample-based timing drifts with CPU load
- ❌ Beat drops during graph reloads
- ❌ No recovery from underruns
- ❌ Time lost forever if samples are skipped
- ❌ No way to reset cycles (like Tidal's resetCycles)

## Solution: Wall-Clock Based Timing

Like Tidal Cycles, maintain a **global clock** independent of processing:

### Architecture

```rust
pub struct UnifiedSignalGraph {
    // ... existing fields ...

    /// Session start time (wall-clock)
    session_start_time: std::time::Instant,

    /// Cycle offset for resetCycles command
    /// cycle_position = (now - session_start_time).as_secs_f64() * cps + cycle_offset
    cycle_offset: f64,

    /// Cycles per second (tempo)
    cps: f32,
}
```

### Clock Calculation

```rust
pub fn get_cycle_position(&self) -> f64 {
    let elapsed = self.session_start_time.elapsed().as_secs_f64();
    elapsed * self.cps as f64 + self.cycle_offset
}
```

### Benefits

✅ **Never drifts** - always based on wall-clock time
✅ **No beat drops** - graph reloads don't affect clock
✅ **Recovers from underruns** - clock keeps ticking
✅ **Can reset cycles** - just adjust `cycle_offset`
✅ **Sample-accurate** - calculate expected sample time from clock

### Migration Strategy

1. Add `session_start_time` and `cycle_offset` to `UnifiedSignalGraph`
2. Replace `cycle_position` increment with wall-clock calculation
3. Update `set_cycle_position()` to set `cycle_offset`
4. Add `reset_cycles()` command
5. Add `set_cycle(n)` command to jump to specific cycle

### Commands

```phonon
-- Reset clock to cycle 0
resetCycles

-- Jump to cycle 5.3
setCycle 5.3

-- Current tempo (cycles per second)
tempo: 2.0
```

### Implementation Notes

**During audio processing:**
```rust
fn process_sample(&mut self) -> f32 {
    // Get current cycle position from wall-clock
    let cycle_position = self.get_cycle_position();

    // Process audio at this cycle position
    // ...

    // NO increment needed - clock is wall-clock based!
}
```

**During graph reload:**
```rust
// Old approach (loses time):
// let old_cycle = old_graph.get_cycle_position();
// new_graph.set_cycle_position(old_cycle);

// New approach (wall-clock always correct):
// new_graph inherits session_start_time → clock automatically synced!
```

**Transfer on reload:**
```rust
// Transfer session timing from old graph
new_graph.session_start_time = old_graph.session_start_time;
new_graph.cycle_offset = old_graph.cycle_offset;
// Clock seamlessly continues!
```

## Tidal Cycles Reference

Tidal uses `LogicalTime` and `LogicalNow`:
- `logicalTime` = absolute time since session start
- `logicalNow` = current logical time
- `resetCycles` sets logical offset to reset to cycle 0

Source: https://github.com/tidalcycles/Tidal/blob/main/src/Sound/Tidal/Context.hs#L89

## Implementation Phases

### Phase 1: Basic Wall-Clock (This Session)
- Add `session_start_time: Instant` to `UnifiedSignalGraph`
- Add `cycle_offset: f64`
- Replace increment with wall-clock calculation
- Update graph reload to transfer timing state

### Phase 2: Reset Commands
- Add `resetCycles` command
- Add `setCycle n` command
- Add `nudge` command (shift timing by small amount)

### Phase 3: Link Mode (Future)
- Ableton Link integration for multi-device sync
- Quantum (bars before playback starts)
- BPM sync across devices

## Testing

Test that:
1. Clock continues during graph reload (no beat drop)
2. Underruns don't lose time (clock catches up)
3. CPU load doesn't affect timing (wall-clock stable)
4. resetCycles actually resets to cycle 0
5. setCycle jumps to correct position

Use onset detection from test_tempo_verification.rs to verify timing accuracy.
