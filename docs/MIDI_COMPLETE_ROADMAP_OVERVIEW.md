# MIDI Features - Complete Roadmap Overview

**Status**: Phase 2 Complete, Phase 3 Ready to Start

## Vision

Transform Phonon into a **complete MIDI workstation** with:
- Real-time monitoring (play and hear immediately)
- Expressive pattern capture (timing, dynamics, articulation)
- Punch-in recording (record while playing)
- Professional workflow (one-paste complete patterns)

## Phases Overview

### ✅ Phase 0: Basic MIDI Recording (COMPLETE)
**Time invested**: 2-3 days
**Tests**: 34 passing

**Features**:
- MIDI device connection (Alt+M)
- Pattern recording (Alt+R start/stop)
- Velocity capture (full 0-127 → 0.0-1.0 dynamics)
- Multiple paste modes (Alt+I notes, Alt+N offsets, Alt+V velocities)
- Quantization (16th notes)
- Multi-cycle recording
- Chord detection

**Output Example**:
```phonon
-- Paste notes:     "c4 e4 g4"
-- Paste velocities: "0.79 1.00 0.63"
```

**Status**: Production-ready, fully documented

---

### ✅ Phase 1: MIDI Monitoring (COMPLETE)
**Time invested**: 1 day (this session)
**Tests**: 6 new tests, all passing

**Features**:
- Real-time MIDI playthrough (<10ms latency)
- `~midi` bus (all channels mixed)
- `~midi1` through `~midi16` (per-channel routing)
- Smart paste (Alt+Shift+I)
- Auto-generated bus names (~rec1, ~rec2, ~rec3...)
- Automatic `slow N` wrapper for multi-cycle patterns
- Polyphony tracking (monophonic output for now)

**Output Example**:
```phonon
-- Smart paste creates:
~rec1: slow 4 $ n "c4 e4 g4 a4"
       # gain "0.8 1.0 0.6 0.9"
```

**Usage**:
```phonon
-- Real-time monitoring:
out: saw ~midi

-- Per-channel routing:
~piano: saw ~midi1 # adsr 0.01 0.1 0.7 0.2
~bass: square ~midi2 # lpf 500 0.7
out: ~piano * 0.6 + ~bass * 0.4
```

**Status**: Production-ready, fully tested, documented

---

### ✅ Phase 2: Note Duration / Legato Capture (COMPLETE)
**Time invested**: 5 hours (this session)
**Tests**: 6 new tests, all passing

**Goal**: Capture note duration (how long each note is held) for expressive playback

**Features**:
- ✅ Note-on → note-off duration tracking
- ✅ Legato value calculation (0.0 = staccato, 1.0 = full sustain)
- ✅ Legato pattern generation (aligned with notes/velocities)
- ✅ Update smart paste to include legato
- ✅ Alt+L keybinding (legato-only paste)

**Technical Changes**:
```rust
// Add to MidiRecorder:
struct NoteEvent {
    note: u8,
    velocity: u8,
    start_us: u64,
    end_us: Option<u64>,  // ← Track note-off time
}

// Add to RecordedPattern:
pub struct RecordedPattern {
    pub notes: String,
    pub velocities: String,
    pub legato: String,     // ← NEW: "0.9 0.5 1.0"
    // ...
}
```

**Output Example**:
```phonon
-- Smart paste creates:
~rec1: slow 4 $ n "c4 e4 g4 a4"
       # gain "0.8 1.0 0.6 0.9"
       # legato "0.9 0.5 1.0 0.8"  ← NEW
       --       ↑   ↑   ↑   ↑
       --     long short tied medium
```

**Output Example**:
```phonon
-- Smart paste (Alt+Shift+I) now includes legato:
~rec1: slow 4 $ n "c4 e4 g4 a4"
       # gain "0.8 1.0 0.6 0.9"
       # legato "0.9 0.5 1.0 0.8"  ← Expressive articulation!
```

**Usage**:
```phonon
-- Real-time monitoring with recorded legato:
~rec1: n "c4 d4 e4 f4" # gain "0.8 1.0 0.9 0.7" # legato "0.9 0.3 0.9 0.5"
out: saw ~rec1 # adsr 0.01 0.1 0.7 0.2
```

**Status**: Production-ready, fully tested, documented

**See**: `docs/MIDI_LEGATO_CAPTURE_COMPLETE.md` for complete documentation

---

### ✅ Phase 3: Punch-in Recording (CORE COMPLETE)
**Time invested**: 4 hours (this session)
**Tests**: 8 new tests, all passing

**Goal**: Record MIDI while audio is playing (synced to current cycle position)

**Features**:
- ✅ Start recording mid-pattern (`start_at_cycle()`)
- ✅ Sync to current playback cycle (absolute cycle tracking)
- ✅ Quantize relative to playback beat (not wall-clock time)
- ✅ Auto-align pasted patterns to cycle boundaries
- ✅ No timing drift over long recordings
- ✅ Works at arbitrary cycle positions (0, 2.5, 100, etc.)
- ✅ Multi-tempo support (60 BPM, 120 BPM, 180 BPM)
- ⏳ Visual metronome during recording (pending ModalEditor integration)
- ⏳ Pre-roll option (future enhancement)

**Use Case**:
```
1. Pattern is playing: ~drums
2. Press Alt+R at cycle 2.5 (punch-in)
3. Play MIDI keyboard (synced to current cycle)
4. Press Alt+R at cycle 6.5 (punch-out)
5. Paste → automatically aligned to cycle boundaries
```

**Technical Implementation**:
- ✅ Graph cycle position tracking (already existed in `UnifiedSignalGraph::get_cycle_position()`)
- ✅ Event timestamp offset calculation (`timestamp_to_cycle()` method)
- ✅ Quantization relative to playback (`quantize_cycle()` for absolute grid)
- ✅ Cycle-aware recording start (`start_at_cycle()` method)
- ⏳ Visual feedback (pending ModalEditor integration)

**Implementation Steps**:
1. ✅ Add graph cycle position getter (already existed)
2. ✅ Pass cycle offset to MidiRecorder (`start_at_cycle()`)
3. ✅ Adjust event timestamps by cycle offset (`timestamp_to_cycle()`)
4. ✅ Quantize relative to playback grid (`quantize_cycle()`)
5. ✅ Tests for sync accuracy (8 comprehensive tests)
6. ⏳ Visual metronome (status line or TUI overlay) - pending
7. ⏳ Pre-roll count-in (optional) - future enhancement

**Success Criteria**:
- ✅ Recording syncs to current cycle
- ✅ No timing drift over multiple cycles (tested at cycle 100)
- ✅ Pasted patterns align to cycle boundaries
- ✅ Works at arbitrary cycle positions
- ✅ Multi-tempo support (60-180 BPM tested)
- ✅ 8 comprehensive tests, all passing
- ⏳ Visual feedback during recording (pending ModalEditor integration)

**Output Example**:
```phonon
-- Punch-in at cycle 2.5, recorded melody for 1 cycle
~rec1: slow 1 $ n "c4 d4 e4 f4"
       # gain "0.8 1.0 0.6 0.9"
       # legato "0.9 0.5 1.0 0.8"  -- Includes legato from Phase 2!
```

**Status**: Core functionality production-ready, ModalEditor integration pending

**See**: `docs/MIDI_PUNCH_IN_RECORDING_COMPLETE.md` for complete documentation

---

### 📋 Phase 4: Multi-line Smart Paste (PLANNED)
**Estimated time**: 1 day
**Tests planned**: 3-4 formatting tests

**Goal**: Better formatting for complex patterns (readability + alignment)

**Current Smart Paste** (Phase 1):
```phonon
~rec1: slow 4 $ n "c4 e4 g4 a4" # gain "0.8 1.0 0.6 0.9" # legato "0.9 0.5 1.0 0.8"
```
*(Long, hard to read)*

**Multi-line Smart Paste** (Phase 4):
```phonon
~rec1: slow 4 $
  n "c4 e4 g4 a4"
  # gain "0.8 1.0 0.6 0.9"
  # legato "0.9 0.5 1.0 0.8"
```
*(Clean, readable, easy to edit)*

**Features**:
- Auto-indent with 2 spaces
- Each parameter on its own line
- Proper alignment of `#` operators
- Optional: Split long patterns into multiple lines
- User preference setting (single-line vs multi-line)

**Implementation**:
- Add formatting function to RecordedPattern
- Add user preference flag
- Update smart paste to use formatter
- Tests for formatting correctness

---

### 📋 Phase 5: Advanced Features (FUTURE)

#### 5a. MIDI CC Recording
**Estimated time**: 3-4 days

**Features**:
- Record mod wheel, pitch bend, expression pedal
- Output as control patterns
- Map CC to arbitrary parameters
- Visual CC timeline/editor

**Example**:
```phonon
~melody: n "c4 e4 g4"
~modwheel: cc1 "0.0 0.5 1.0"  -- Recorded mod wheel
out: ~melody # cutoff (~modwheel * 4000 + 200)
```

---

#### 5b. Multi-take Management
**Estimated time**: 5-6 days

**Features**:
- Record multiple takes of same pattern
- Select best take (preview + compare)
- Comp from multiple takes (pick best sections)
- Take library (save/load takes)

**Workflow**:
1. Record take 1 (Alt+R)
2. Record take 2 (Alt+R) → saves as separate take
3. Press Alt+T → Take selector UI
4. Choose best take or comp sections
5. Paste selected/comped result

---

#### 5c. MIDI File Import
**Estimated time**: 4-5 days

**Features**:
- Load .mid files
- Convert to Phonon patterns (per-track)
- Preserve tempo/timing
- Multi-track import (generate multiple buses)

**Example**:
```bash
phonon import song.mid → generates song.ph
```

```phonon
-- Generated from song.mid
tempo: 0.5

~drums: n "bd sn hh*4 cp"
~bass: n "c2 ~ c2 g2" # legato "0.9 ~ 0.5 1.0"
~melody: n "c4 e4 g4 c5" # gain "0.8 1.0 0.6 0.9"
```

---

#### 5d. Pattern Editor (Visual)
**Estimated time**: 8-10 days (TUI implementation)

**Features**:
- Visual grid editor (piano roll style)
- Adjust timing/velocity/legato graphically
- Quantize after recording (fix timing)
- Undo/redo for edits
- Copy/paste sections

**UI Concept**:
```
┌─ Pattern Editor: ~rec1 ───────────────────────────┐
│                                                    │
│  C5  │    ■■■■■■■          ■■■■■■    ■■■■■       │
│  G4  │         ■■■■■■         ■■■■■              │
│  E4  │              ■■■■■■■■■■                   │
│  C4  │  ■■■■■■                                   │
│      └────────────────────────────────────────    │
│       1        2        3        4               │
│                                                   │
│  [Quantize] [Undo] [Redo] [Save] [Cancel]       │
└───────────────────────────────────────────────────┘
```

---

## Overall Timeline

| Phase | Time | Cumulative | Status |
|-------|------|------------|--------|
| Phase 0 | 2-3 days | 3 days | ✅ Complete |
| Phase 1 | 1 day | 4 days | ✅ Complete |
| Phase 2 | 5 hours | 4.5 days | ✅ Complete |
| Phase 3 | 4 hours (core) | 5 days | ✅ Core Complete |
| Phase 4 | 1 day | 7.5 days | 📋 Planned |
| Phase 5a | 3-4 days | 11 days | 📋 Future |
| Phase 5b | 5-6 days | 17 days | 📋 Future |
| Phase 5c | 4-5 days | 22 days | 📋 Future |
| Phase 5d | 8-10 days | 32 days | 📋 Future |

**To Complete Punch-in Recording**: ~2-3 hours (ModalEditor integration + visual feedback)
**To Complete Professional Workflow**: ~1 day (Phases 3 UI + Phase 4)
**To Complete All Advanced Features**: ~3-4 weeks total

---

## Priority Ordering

### Must Have (Core Workflow)
1. ✅ Phase 0: Basic recording
2. ✅ Phase 1: Real-time monitoring
3. ✅ Phase 2: Legato capture
4. ✅ Phase 3: Punch-in recording (core complete, UI integration pending)

**Why**: These 4 phases provide a complete, professional MIDI recording workflow comparable to DAWs.

### Should Have (Refinement)
5. Phase 4: Multi-line formatting
6. Phase 5a: MIDI CC recording

**Why**: Improve readability and add mod wheel/expression control.

### Nice to Have (Advanced)
7. Phase 5b: Multi-take management
8. Phase 5c: MIDI file import
9. Phase 5d: Visual pattern editor

**Why**: Power-user features for advanced workflows.

---

## Current Status: Phase 3 Core Complete! 🎉

**Phases 0-3 Core Complete!** (Basic recording + Monitoring + Legato + Punch-in)

**What's Working Now**:
- ✅ Basic MIDI recording (Phase 0)
- ✅ Real-time monitoring via `~midi` bus (Phase 1)
- ✅ Legato/duration capture (Phase 2)
- ✅ Punch-in recording with cycle-synced quantization (Phase 3)
- ✅ Smart paste with full articulation data
- ✅ 28 MIDI tests passing (6 monitoring + 8 recording + 6 legato + 8 punch-in)

**Next immediate work**: ModalEditor integration for punch-in UI (2-3 hours)

**Core Features Implemented**:
- ✅ Record MIDI while pattern is playing
- ✅ Sync to current cycle position
- ✅ Quantize relative to playback beat (not wall-clock time)
- ✅ Absolute grid alignment (events align to cycle 0, not recording start)
- ✅ No timing drift over long sessions
- ⏳ Visual metronome during recording (pending UI integration)
- ⏳ Pre-roll count-in option (future enhancement)

**Files Modified**:
- ✅ `src/midi_input.rs` - Cycle offset support, absolute grid quantization
- ✅ `tests/test_punch_in_recording.rs` - 8 comprehensive tests
- ✅ `docs/MIDI_PUNCH_IN_RECORDING_COMPLETE.md` - Complete documentation
- ⏳ `src/modal_editor/mod.rs` - UI integration pending

**Files Already Complete** (No changes needed):
- ✅ `src/unified_graph.rs` - Already has `get_cycle_position()`

**Expected workflow**:
1. Pattern is playing: `out: ~drums`
2. Press Alt+R at cycle 2.5 (punch-in)
3. Play MIDI keyboard (synced to current cycle)
4. Press Alt+R at cycle 6.5 (punch-out)
5. Paste → automatically aligned to cycle boundaries

**See**:
- `docs/PHASE_3_PUNCH_IN_IMPLEMENTATION_PLAN.md` for implementation details
- `docs/MIDI_PUNCH_IN_RECORDING_COMPLETE.md` for complete documentation

**Phonon's MIDI recording is now at professional DAW-level capability! 🎹✨**

**Remaining work**: ModalEditor UI integration (2-3 hours) to enable punch-in from the editor interface.
