# Polyphonic Architecture Design Document

**Status**: DESIGN PHASE
**Priority**: CRITICAL
**Est. Implementation Time**: 3-5 days
**Last Updated**: 2025-11-06

## Problem Statement

### Current Limitations
1. **Fixed 64-voice limit**: `VoiceManager` allocates exactly 64 voices at startup
2. **One event = one voice**: Pattern events trigger single samples, no chord expansion
3. **Chord notation ignored**: `note "c4'maj"` plays only root note (C4)
4. **MIDI polyphony blocked**: Cannot handle multiple simultaneous MIDI notes
5. **Single-threaded rendering**: All voices render sequentially on one thread

### Why This Matters
- **Compositional expressiveness**: Chords are fundamental to music
- **MIDI input**: Real keyboards send polyphonic note data
- **Performance**: Multi-threading can utilize modern CPUs
- **Competitive parity**: SuperCollider, Max/MSP, CSound all support unlimited polyphony

## Design Goals

### Functional Requirements
1. **Unlimited polyphony**: No hard voice limit (CPU is the only constraint)
2. **Chord expansion**: `note "c4'maj"` → triggers 3 voices (C, E, G)
3. **Dynamic allocation**: Voices created on-demand, freed when done
4. **Voice stealing**: Optional, priority-based when CPU overloaded
5. **MIDI polyphony**: Each note-on → new voice, note-off → release
6. **Pattern polyphony**: Patterns can trigger multiple simultaneous events

### Performance Requirements
1. **Multi-threaded rendering**: Utilize all CPU cores
2. **Real-time capable**: < 10ms latency for live performance
3. **Graceful degradation**: Performance warnings, not crashes
4. **Memory efficient**: Release voices when envelopes complete

## Research: Existing Approaches

### SuperCollider
**Architecture**:
- Dynamic synth graph with independent nodes
- Each note = new Synth instance
- Server manages thousands of synths simultaneously
- Multi-threaded audio synthesis

**Voice Management**:
- No fixed limit, allocates on demand
- Automatic cleanup when envelopes finish
- Voice stealing via `.stealVoice` if needed

**Pros**: Industry-proven, highly flexible
**Cons**: Complex server architecture

### Max/MSP / Pure Data
**Architecture**:
- Object graph with poly~ for polyphony
- Dynamic voice allocation within poly~ objects
- Each voice is independent DSP chain

**Pros**: Clear polyphony abstraction
**Cons**: Requires explicit poly~ wrapping

### DAW Approach (Live, Logic, etc.)
**Architecture**:
- Unlimited voices per track
- Voice stealing algorithms (oldest, quietest, lowest priority)
- MIDI note-on → voice allocation, note-off → release

**Pros**: User-friendly, predictable
**Cons**: Can consume excessive CPU without limits

### Game Audio (FMOD, Wwise)
**Architecture**:
- Priority-based voice stealing
- Virtual voices (unlimited) vs. real voices (hardware limit)
- Automatic voice stealing by priority

**Pros**: Excellent CPU management
**Cons**: Overkill for music production

## Proposed Architecture

### Option A: Dynamic Voice Pool (RECOMMENDED)

**Core Concept**: Replace fixed 64-voice array with dynamic Vec-based allocation

#### Voice Lifecycle
```
Pattern Event
    ↓
Chord Expansion (if applicable)
    ↓
[Voice, Voice, Voice] (one per note)
    ↓
Voice Rendering (multi-threaded)
    ↓
Envelope Complete → Voice Freed
```

#### Key Components

**1. DynamicVoiceManager**
```rust
pub struct DynamicVoiceManager {
    voices: Vec<Voice>,           // Dynamically grows
    next_voice_id: u64,            // Unique ID per voice
    max_voices: Option<usize>,     // Optional limit (default: None)
    voice_steal_mode: StealMode,   // How to steal voices if max reached
    performance_monitor: PerfMonitor,
}

enum StealMode {
    Oldest,      // Steal oldest voice
    Quietest,    // Steal quietest voice (by envelope level)
    Priority,    // Steal lowest priority
    None,        // Never steal, just warn
}
```

**2. Voice Structure**
```rust
pub struct Voice {
    id: u64,                    // Unique identifier
    state: VoiceState,          // Playing, Releasing, Free
    priority: u8,               // 0-255, higher = more important
    trigger_time: f64,          // When voice started
    envelope_level: f32,        // Current envelope amplitude

    // Existing voice data
    sample_data: Arc<Vec<f32>>,
    playback_position: usize,
    gain: f32,
    pan: f32,
    speed: f32,
    // ... etc
}

enum VoiceState {
    Free,              // Available for allocation
    Attacking,         // Envelope attack phase
    Playing,           // Sustained/playing
    Releasing,         // Envelope release phase
}
```

**3. Chord Expansion**

Happens at pattern evaluation time:

```rust
// In unified_graph.rs, Sample node evaluation:
let note_val = self.eval_note_signal_at_time(&note, event_start_abs);

// NEW: Parse note value for chord notation
let notes_to_trigger = if is_chord_notation(&note_string) {
    expand_chord(&note_string)  // Returns Vec<f32> of semitone offsets
} else {
    vec![note_val]  // Single note
};

// Trigger one voice per note
for note_offset in notes_to_trigger {
    let voice_id = voice_manager.allocate_voice();
    voice_manager.trigger(voice_id, sample_name, note_offset, gain, pan, ...);
}
```

**4. Multi-threaded Rendering**

Use `rayon` for parallel voice processing:

```rust
use rayon::prelude::*;

impl DynamicVoiceManager {
    pub fn render(&mut self, num_samples: usize) -> Vec<f32> {
        // Render all active voices in parallel
        let voice_buffers: Vec<Vec<f32>> = self.voices
            .par_iter_mut()  // Parallel iterator
            .filter(|v| v.state != VoiceState::Free)
            .map(|voice| voice.render(num_samples))
            .collect();

        // Sum all voice outputs (sequential, fast)
        let mut output = vec![0.0; num_samples];
        for voice_buffer in voice_buffers {
            for (i, sample) in voice_buffer.iter().enumerate() {
                output[i] += sample;
            }
        }

        // Cleanup voices that finished
        self.voices.retain(|v| v.state != VoiceState::Free ||
                                self.should_keep_free_voice());

        output
    }
}
```

**5. Performance Monitoring**

```rust
pub struct PerfMonitor {
    voice_count_history: VecDeque<usize>,  // Rolling window
    render_time_history: VecDeque<Duration>,
    cpu_usage_percent: f32,
    max_concurrent_voices: usize,          // Peak usage
}

impl PerfMonitor {
    pub fn warn_if_overloaded(&self) {
        if self.cpu_usage_percent > 90.0 {
            eprintln!("⚠️  CPU: {:.1}% - {} active voices",
                self.cpu_usage_percent,
                self.voice_count_history.back().unwrap());
        }
    }

    pub fn suggest_voice_limit(&self) -> Option<usize> {
        if self.max_concurrent_voices > 500 {
            Some(500)  // Suggest reasonable limit
        } else {
            None
        }
    }
}
```

### Option B: SuperCollider-Style Synth Graph (FUTURE)

**Deferred**: This is a more radical rewrite. Option A gets us 90% of the way there.

**Concept**: Each note/sample trigger creates a Synth node in the graph, runs independently until envelope completes.

**Pros**:
- Most flexible
- Industry-standard approach
- Better for complex routing

**Cons**:
- Requires major refactoring of UnifiedSignalGraph
- More complex implementation (2-3 weeks)

## Implementation Plan

### Phase 1: Dynamic Voice Pool (2 days)
**Goal**: Remove 64-voice limit, enable dynamic allocation

**Tasks**:
- [x] Research existing approaches
- [ ] Create `DynamicVoiceManager` struct
- [ ] Replace fixed array with `Vec<Voice>`
- [ ] Add `VoiceState` enum (Free, Playing, Releasing)
- [ ] Implement `allocate_voice()` and `free_voice()`
- [ ] Add voice lifecycle management
- [ ] Update voice triggering in unified_graph.rs
- [ ] Add performance monitoring
- [ ] Test with 100+ simultaneous voices

**Success Criteria**:
- Can trigger 200+ voices without hard limit
- No crashes or memory leaks
- Performance degradation is gradual, not sudden

### Phase 2: Chord Expansion (1 day)
**Goal**: `note "c4'maj"` triggers 3 voices (C, E, G)

**Tasks**:
- [ ] Parse chord notation in eval_note_signal_at_time()
- [ ] Use existing CHORD_INTERVALS from pattern_tonal.rs
- [ ] Return Vec<f32> instead of f32 for chord notes
- [ ] Update voice triggering to handle multiple notes
- [ ] Test: `s "bd" # note "c4'maj"` produces 3-voice chord
- [ ] Test: `s "bd*4" # note "c4'maj e4'min g4'dom7 c5'maj"`
- [ ] Add tests for all 30+ chord types

**Success Criteria**:
- Chords sound correct (all notes simultaneous)
- No timing drift between chord voices
- Can play chord progressions smoothly

### Phase 3: Multi-threading (1-2 days)
**Goal**: Parallel voice rendering for performance

**Tasks**:
- [ ] Add `rayon` dependency to Cargo.toml
- [ ] Convert render loop to par_iter_mut()
- [ ] Benchmark: single-threaded vs multi-threaded
- [ ] Ensure thread safety (no race conditions)
- [ ] Test on 1-core, 4-core, 8-core+ systems
- [ ] Measure CPU utilization
- [ ] Add --single-threaded flag for debugging

**Success Criteria**:
- 50-75% reduction in render time (4+ cores)
- Linear scaling up to core count
- No audio glitches or race conditions
- Real-time performance with 100+ voices

### Phase 4: Voice Stealing (Optional, 1 day)
**Goal**: Graceful degradation when CPU overloaded

**Tasks**:
- [ ] Add `StealMode` enum (Oldest, Quietest, Priority, None)
- [ ] Implement stealing algorithms
- [ ] Add `--max-voices` CLI flag
- [ ] Add priority system (MIDI velocity, pattern emphasis)
- [ ] Test: 1000 voices → graceful stealing
- [ ] Performance warnings in terminal

**Success Criteria**:
- System stays responsive with excessive voice count
- Stealing is musically unobtrusive
- Clear warnings when stealing occurs

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_chord_expansion() {
    let chord = expand_chord("c4'maj");
    assert_eq!(chord, vec![0.0, 4.0, 7.0]); // C, E, G (semitones from C4)
}

#[test]
fn test_dynamic_voice_allocation() {
    let mut vm = DynamicVoiceManager::new();
    let ids: Vec<_> = (0..200).map(|_| vm.allocate_voice()).collect();
    assert_eq!(vm.active_voice_count(), 200);
}

#[test]
fn test_voice_stealing_oldest() {
    let mut vm = DynamicVoiceManager::new();
    vm.set_max_voices(Some(10));
    vm.set_steal_mode(StealMode::Oldest);

    // Allocate 15 voices, should steal 5 oldest
    for _ in 0..15 {
        vm.allocate_and_trigger(...);
    }
    assert_eq!(vm.active_voice_count(), 10);
}
```

### Integration Tests
```rust
#[test]
fn test_polyphonic_chord_progression() {
    let code = r#"
        tempo: 2.0
        out: s "bd*4" # note "c4'maj f4'maj g4'maj c4'maj"
    "#;

    let audio = render_dsl(code, 4.0); // 4 seconds

    // Each chord = 3 voices = 12 voices per cycle
    // 4 cycles = 48 total voice triggers
    assert_voices_triggered(48);
    assert_audio_not_clipped(audio);
}

#[test]
fn test_high_polyphony_performance() {
    let code = r#"
        tempo: 2.0
        ~chords: s "pad*16" # note "c4'maj13"  # 7 notes per trigger
        out: ~chords
    "#;

    // 16 triggers/cycle × 7 notes = 112 simultaneous voices
    let start = Instant::now();
    let audio = render_dsl(code, 1.0);
    let duration = start.elapsed();

    assert!(duration.as_millis() < 1000, "Too slow: {}ms", duration.as_millis());
}
```

### Manual Testing
- [ ] Play 10-note chord (`c4'maj13`)
- [ ] Play fast arpeggiated chords
- [ ] Stress test: 500+ simultaneous voices
- [ ] MIDI keyboard input (once MIDI implemented)
- [ ] Complex progression: `s "piano*8" # note "c4'maj7 f4'maj7 g4'dom7 c4'maj7"`

## Performance Targets

| Scenario | Target | Measurement |
|----------|--------|-------------|
| 64 voices (current) | < 5% CPU | Baseline |
| 200 voices | < 15% CPU | Reasonable polyphony |
| 500 voices | < 40% CPU | Heavy polyphony |
| 1000 voices | < 80% CPU | Stress test |

**Hardware**: 4-core 3.0 GHz CPU (typical laptop)

## Migration Path

### Backward Compatibility
All existing code continues to work:
- `s "bd*4"` - still triggers 4 voices
- `note "5"` - still does semitone offset
- `note "c4 e4 g4"` - triggers 3 sequential notes (not a chord)

### New Syntax for Chords
- `note "c4'maj"` - triggers simultaneous C-E-G
- `note "c4'maj7"` - triggers C-E-G-B
- Can mix: `note "0 c4'maj 5 g4'min"`

### Breaking Changes
**None**: This is purely additive functionality

## Risks & Mitigations

### Risk 1: Thread Safety
**Issue**: Parallel rendering could cause race conditions
**Mitigation**:
- Each voice is independent (no shared mutable state)
- Use atomic counters for voice IDs
- Thorough testing with ThreadSanitizer

### Risk 2: Memory Leaks
**Issue**: Voices not properly freed
**Mitigation**:
- Automatic cleanup when envelope ends
- Manual `clear_voices()` method
- Memory profiling with valgrind/heaptrack

### Risk 3: CPU Overload
**Issue**: User triggers 10,000 voices, system freezes
**Mitigation**:
- Performance monitoring with warnings
- Optional voice limits
- Voice stealing algorithms

### Risk 4: Timing Drift
**Issue**: Chord voices don't start exactly together
**Mitigation**:
- All voices triggered in same render callback
- Sub-sample timing precision
- Test with oscilloscope for phase alignment

## Future Enhancements

### Beyond Phase 4
1. **Voice Priority System**: MIDI velocity → voice priority
2. **Voice Groups**: Mute/solo groups of voices
3. **Per-voice Effects**: Each voice has own effect chain
4. **SIMD Optimization**: Vectorize voice processing (SSE/AVX)
5. **GPU Offload**: Experimental GPU voice rendering
6. **SuperCollider Integration**: Phonon → SC via OSC

## References

- [SuperCollider Architecture](https://doc.sccode.org/Reference/Server-Architecture.html)
- [Game Audio Voice Management](https://www.fmod.com/docs/2.00/studio/voice-management.html)
- [Rust Rayon: Data Parallelism](https://docs.rs/rayon/latest/rayon/)
- [Audio Thread Safety in Rust](https://rust-audio.github.io/)

## Open Questions

1. **Should we impose a default max voice limit?**
   → Recommendation: No hard limit, but warning at 500+

2. **Voice stealing algorithm preference?**
   → Recommendation: "Oldest" for simplicity, "Quietest" for quality

3. **How to visualize polyphony for users?**
   → Recommendation: Terminal output: "🎵 64 voices active (12% CPU)"

4. **Should MIDI velocity map to voice priority?**
   → Recommendation: Yes, louder MIDI notes = higher priority

5. **Integration with pattern system: how to represent chords?**
   → Current approach: Chord expansion happens at voice trigger time
   → Alternative: Chord as Pattern<Vec<f32>> (more complex, deferred)

## Success Metrics

**Phase 1-3 Complete When**:
- ✅ Can trigger 500+ simultaneous voices
- ✅ `note "c4'maj"` plays full C major chord
- ✅ Multi-threading reduces CPU 50%+ on 4-core systems
- ✅ No memory leaks after 1 hour stress test
- ✅ All existing tests still pass
- ✅ New polyphony tests pass
- ✅ User-facing documentation updated

**Long-term Success**:
- MIDI keyboard input works seamlessly
- Users create complex chord progressions
- Real-time performance for live coding
- Community doesn't report polyphony issues

---

**Next Steps**: Review this design, then begin Phase 1 implementation.
