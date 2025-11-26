# MIDI Monitoring Implementation Plan

## Goal
Enable real-time MIDI playthrough with <10ms latency

## Architecture Overview

```
┌─────────────────┐
│ MIDI Hardware   │
└────────┬────────┘
         │ USB/Driver
         ↓
┌─────────────────────────────┐
│ MidiInputHandler            │
│ - Receives raw MIDI bytes   │
│ - Parses to MidiEvent       │
│ - Writes to shared queue    │
└────────┬────────────────────┘
         │ Arc<Mutex<VecDeque<MidiEvent>>>
         ↓
┌─────────────────────────────┐
│ MidiInput SignalNode        │
│ - Reads from shared queue   │
│ - Tracks active notes       │
│ - Converts note → frequency │
│ - Outputs current frequency │
└────────┬────────────────────┘
         │ Signal flow
         ↓
┌─────────────────────────────┐
│ Oscillator/Synth Node       │
│ - Receives frequency signal │
│ - Generates audio           │
└────────┬────────────────────┘
         │
         ↓
    Audio Output
```

## Implementation Steps

### Step 1: Shared MIDI Event Queue (30 min)

**Goal**: Create thread-safe queue accessible from both handler and graph

**Files to modify**:
- `src/midi_input.rs` - Add shared queue
- `src/unified_graph.rs` - Store reference to queue

**Implementation**:
```rust
// In midi_input.rs
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

pub type MidiEventQueue = Arc<Mutex<VecDeque<MidiEvent>>>;

impl MidiInputHandler {
    pub fn new_with_queue() -> (Self, MidiEventQueue) {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        // ... create handler that writes to queue
        (handler, queue)
    }
}
```

**Test**:
```rust
#[test]
fn test_midi_event_queue() {
    let (handler, queue) = MidiInputHandler::new_with_queue();
    // Simulate MIDI event
    // Check queue contains event
}
```

---

### Step 2: MidiInput Signal Node (45 min)

**Goal**: Add node type that reads MIDI events and outputs frequency

**Files to modify**:
- `src/unified_graph.rs` - Add to SignalNode enum

**Implementation**:
```rust
pub enum SignalNode {
    // ...existing nodes...

    /// MIDI Input - Receives MIDI events and outputs frequency
    /// Supports polyphony by tracking multiple active notes
    /// channel: None = all channels, Some(0-15) = specific channel
    MidiInput {
        channel: Option<u8>,           // MIDI channel filter
        active_notes: RefCell<HashMap<u8, f32>>,  // note → velocity
        event_queue: MidiEventQueue,   // Shared event queue
        last_freq: RefCell<f32>,       // Current output frequency
    },
}
```

**Note conversion**:
```rust
fn midi_note_to_freq(note: u8) -> f32 {
    // A4 (MIDI 69) = 440 Hz
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}
```

**Test**:
```rust
#[test]
fn test_midi_to_frequency() {
    assert_eq!(midi_note_to_freq(69), 440.0);  // A4
    assert_eq!(midi_note_to_freq(60), 261.63); // C4
}
```

---

### Step 3: Event Processing in Graph (1 hour)

**Goal**: Process MIDI events during graph traversal

**Files to modify**:
- `src/unified_graph.rs` - eval_node() for MidiInput

**Implementation**:
```rust
// In eval_node()
SignalNode::MidiInput { channel, active_notes, event_queue, last_freq } => {
    // 1. Drain events from queue
    if let Ok(mut queue) = event_queue.lock() {
        while let Some(event) = queue.pop_front() {
            // 2. Filter by channel if specified
            if let Some(ch) = channel {
                if event.channel != *ch {
                    continue;
                }
            }

            // 3. Update active notes
            match event.message_type {
                MidiMessageType::NoteOn { note, velocity } if velocity > 0 => {
                    active_notes.borrow_mut().insert(note, velocity as f32 / 127.0);
                }
                MidiMessageType::NoteOff { note, .. } |
                MidiMessageType::NoteOn { note, velocity: 0 } => {
                    active_notes.borrow_mut().remove(&note);
                }
                _ => {}
            }
        }
    }

    // 4. Get highest active note (or last note if none)
    let freq = if let Some(&note) = active_notes.borrow().keys().max() {
        midi_note_to_freq(note)
    } else {
        *last_freq.borrow()
    };

    *last_freq.borrow_mut() = freq;
    freq
}
```

**Note**: Using highest note for monophonic mode. Polyphony comes later with voice manager integration.

---

### Step 4: Compiler Support for ~midi Buses (30 min)

**Goal**: Recognize ~midi and ~midi1-~midi16 in parser

**Files to modify**:
- `src/compositional_compiler.rs` - Add MIDI bus handling

**Implementation**:
```rust
fn compile_bus_reference(ctx: &mut CompilerContext, name: &str) -> Result<NodeId, String> {
    // Check if it's a MIDI bus
    if name == "midi" {
        // All channels
        return create_midi_input_node(ctx, None);
    }

    if name.starts_with("midi") && name.len() > 4 {
        if let Ok(channel) = name[4..].parse::<u8>() {
            if channel >= 1 && channel <= 16 {
                // Specific channel (convert 1-indexed to 0-indexed)
                return create_midi_input_node(ctx, Some(channel - 1));
            }
        }
    }

    // Otherwise normal bus lookup
    ctx.get_or_create_bus(name)
}

fn create_midi_input_node(ctx: &mut CompilerContext, channel: Option<u8>) -> Result<NodeId, String> {
    let queue = ctx.midi_event_queue.clone()
        .ok_or("MIDI input not available")?;

    Ok(ctx.graph.add_node(SignalNode::MidiInput {
        channel,
        active_notes: RefCell::new(HashMap::new()),
        event_queue: queue,
        last_freq: RefCell::new(440.0),
    }))
}
```

**Test**:
```rust
#[test]
fn test_parse_midi_bus() {
    let code = "out: ~midi # saw 440";
    // Should compile without error
    // Should create MidiInput node
}
```

---

### Step 5: Wire to Modal Editor (30 min)

**Goal**: Connect MIDI handler queue to graph

**Files to modify**:
- `src/modal_editor/mod.rs` - Pass queue to graph

**Implementation**:
```rust
pub struct ModalEditor {
    // ...existing fields...
    midi_event_queue: Option<MidiEventQueue>,
}

impl ModalEditor {
    pub fn new() -> Self {
        let (midi_handler, queue) = MidiInputHandler::new_with_queue();

        Self {
            midi_input: Some(midi_handler),
            midi_event_queue: Some(queue.clone()),
            // ... other fields
        }
    }

    fn compile_code(&mut self, code: &str) -> Result<(), String> {
        let mut ctx = CompilerContext::new();
        ctx.midi_event_queue = self.midi_event_queue.clone();

        // ... rest of compilation
    }
}
```

---

### Step 6: Testing & Verification (1 hour)

**Tests to write**:

**6a. Basic MIDI triggering**:
```rust
#[test]
fn test_midi_triggers_oscillator() {
    let (handler, queue) = MidiInputHandler::new_with_queue();
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // Create ~midi # saw node
    let midi_node = graph.add_node(SignalNode::MidiInput {
        channel: None,
        active_notes: RefCell::new(HashMap::new()),
        event_queue: queue.clone(),
        last_freq: RefCell::new(440.0),
    });

    let osc = graph.add_node(SignalNode::Oscillator {
        freq: Signal::Node(midi_node),
        waveform: Waveform::Saw,
        // ...
    });

    graph.set_output(osc);

    // Simulate MIDI note-on (C4 = 261.63 Hz)
    queue.lock().unwrap().push_back(MidiEvent {
        message_type: MidiMessageType::NoteOn { note: 60, velocity: 100 },
        channel: 0,
        timestamp_us: 0,
        message: vec![0x90, 60, 100],
    });

    // Render and verify frequency change
    let buffer = graph.render(1024);

    // Should hear ~261 Hz sine wave
    let detected_freq = detect_primary_frequency(&buffer, 44100.0);
    assert!((detected_freq - 261.63).abs() < 5.0);
}
```

**6b. Polyphony test**:
```rust
#[test]
fn test_midi_polyphony() {
    // Send C4, E4, G4 (C major chord)
    // Verify all three notes are tracked
    // (Will use highest note for now, full polyphony later)
}
```

**6c. Channel filtering**:
```rust
#[test]
fn test_midi_channel_filtering() {
    // Create ~midi1 and ~midi2
    // Send note on channel 1
    // Verify only ~midi1 receives it
}
```

**6d. Latency measurement**:
```rust
#[test]
fn test_midi_latency() {
    // Timestamp when MIDI event sent
    // Timestamp when audio contains that frequency
    // Verify delta < 10ms (at 44.1kHz, ~441 samples)
}
```

---

### Step 7: Smart Paste Feature (1 hour)

**Goal**: Auto-generate ~rec1, ~rec2 when pasting

**Files to modify**:
- `src/modal_editor/mod.rs` - Add recording counter, smart paste function

**Implementation**:
```rust
pub struct ModalEditor {
    // ...
    recording_counter: usize,  // Tracks ~rec1, ~rec2, etc.
}

impl ModalEditor {
    fn insert_midi_smart_paste(&mut self) {
        if let Some(ref pattern) = self.midi_recorded_pattern.clone() {
            if let Some(ref velocity) = self.midi_recorded_velocity.clone() {
                self.recording_counter += 1;
                let rec_name = format!("~rec{}", self.recording_counter);

                let slow_wrapper = if self.midi_recorded_cycles > 1 {
                    format!("slow {} $ ", self.midi_recorded_cycles)
                } else {
                    String::new()
                };

                let full_pattern = format!(
                    "{}: {}n \"{}\"\n       # gain \"{}\"",
                    rec_name,
                    slow_wrapper,
                    pattern,
                    velocity
                );

                // Insert at cursor
                for c in full_pattern.chars() {
                    self.insert_char(c);
                }

                self.status_message = format!("📝 Inserted {} with full dynamics", rec_name);
            }
        }
    }
}
```

**Keybinding**: `Alt+Shift+I` or `Alt+P` for "paste complete"

---

### Step 8: Legato Capture (2 hours)

**Goal**: Track note duration, output as legato parameter

**Implementation**:
- Track note-on → note-off duration
- Normalize to grid (0.0 = very short, 1.0 = full sustain)
- Add to RecordedPattern struct
- Include in smart paste

**Details**: See Phase 2 in MIDI_MONITORING_ROADMAP.md

---

## Timeline Estimate

| Step | Time | Cumulative |
|------|------|------------|
| 1. Event queue | 30 min | 30 min |
| 2. MidiInput node | 45 min | 1h 15min |
| 3. Event processing | 1 hour | 2h 15min |
| 4. Compiler support | 30 min | 2h 45min |
| 5. Wire to editor | 30 min | 3h 15min |
| 6. Testing | 1 hour | 4h 15min |
| 7. Smart paste | 1 hour | 5h 15min |
| 8. Legato capture | 2 hours | 7h 15min |

**Total**: ~7-8 hours of focused work

## Testing Strategy

### Unit Tests
- MIDI event queue operations
- Note → frequency conversion
- Channel filtering
- Active note tracking

### Integration Tests
- End-to-end MIDI → audio
- Multi-channel routing
- Polyphony handling
- Latency measurement

### Manual Testing
```bash
# 1. Launch editor
cargo run --release --bin phonon -- edit

# 2. Create MIDI monitoring patch
~piano: ~midi1 # saw 440 # adsr 0.01 0.1 0.7 0.2
out: ~piano

# 3. Play MIDI keyboard
# 4. Verify real-time audio output
# 5. Record pattern (Alt+R)
# 6. Paste with smart paste (Alt+Shift+I)
```

## Success Criteria

- ✅ Hear MIDI keyboard in real-time (<10ms latency)
- ✅ All 16 MIDI channels work independently
- ✅ Polyphony tracked (even if monophonic output initially)
- ✅ Smart paste generates ~rec1, ~rec2, etc.
- ✅ Full dynamics captured (gain pattern)
- ✅ Note duration captured (legato pattern)
- ✅ No audio glitches or dropouts
- ✅ Works with all existing synth nodes

## Current Status

**Starting now!** 🚀
