# Next Steps: Integrating SynthPattern Node

## ✅ Completed

1. **Created SynthVoiceManager** (`src/synth_voice_manager.rs`)
   - 64 polyphonic voices
   - Per-voice ADSR envelopes
   - Oscillator waveforms: Sine, Saw, Square, Triangle
   - Voice stealing
   - All tests passing (4/4)

## 🔧 Remaining Work

### Step 1: Add SignalNode::SynthPattern variant

In `src/unified_graph.rs` after the `Sample` node (around line 418):

```rust
/// Pattern-triggered synthesizer
SynthPattern {
    pattern_str: String,
    pattern: Pattern<String>,
    last_trigger_time: f32,
    waveform: Waveform,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    gain: Signal,
    pan: Signal,
},
```

### Step 2: Add synth_voice_manager to UnifiedSignalGraph

In `src/unified_graph.rs` in the `UnifiedSignalGraph` struct (around line 327):

```rust
/// Voice manager for polyphonic sample playback
voice_manager: RefCell<VoiceManager>,

/// Synth voice manager for polyphonic synthesis
synth_voice_manager: RefCell<SynthVoiceManager>,

/// Sample counter for debugging
sample_count: usize,
```

And initialize it in `new()` (around line 347):

```rust
voice_manager: RefCell::new(VoiceManager::new()),
synth_voice_manager: RefCell::new(SynthVoiceManager::new(sample_rate)),
sample_count: 0,
```

### Step 3: Add evaluation logic for SynthPattern

In `src/unified_graph.rs` in the `eval_node()` match statement, add after the `Sample` case:

```rust
SignalNode::SynthPattern {
    pattern,
    last_trigger_time,
    waveform,
    attack,
    decay,
    sustain,
    release,
    gain,
    pan,
    ..
} => {
    use crate::pattern_tonal::{note_to_midi, midi_to_freq};
    use crate::synth_voice_manager::{SynthWaveform, ADSRParams};

    // Evaluate DSP parameters
    let gain_val = self.eval_signal(&gain).max(0.0).min(10.0);
    let pan_val = self.eval_signal(&pan).clamp(-1.0, 1.0);

    // Query pattern for note events
    let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(self.cycle_position),
            Fraction::from_float(self.cycle_position + sample_width),
        ),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);

    let last_event_start = if let Some(Some(SignalNode::SynthPattern { last_trigger_time: lt, .. })) = self.nodes.get(node_id.0) {
        *lt as f64
    } else {
        -1.0
    };

    let mut latest_triggered_start = last_event_start;

    // Trigger synth voices for new note events
    for event in events.iter() {
        let note_name = event.value.trim();

        // Skip rests
        if note_name == "~" || note_name.is_empty() {
            continue;
        }

        // Get event start time
        let event_start_abs = if let Some(whole) = &event.whole {
            whole.begin.to_float()
        } else {
            event.part.begin.to_float()
        };

        // Only trigger NEW events
        let tolerance = sample_width * 0.001;
        let event_is_new = event_start_abs > last_event_start + tolerance;

        if event_is_new {
            // Parse note name to frequency
            let frequency = if let Ok(numeric) = note_name.parse::<f32>() {
                numeric
            } else if let Some(midi) = note_to_midi(note_name) {
                midi_to_freq(midi) as f32
            } else {
                440.0  // Default to A4
            };

            // Convert Waveform to SynthWaveform
            let synth_waveform = match waveform {
                Waveform::Sine => SynthWaveform::Sine,
                Waveform::Saw => SynthWaveform::Saw,
                Waveform::Square => SynthWaveform::Square,
                Waveform::Triangle => SynthWaveform::Triangle,
            };

            // ADSR parameters
            let adsr = ADSRParams {
                attack,
                decay,
                sustain,
                release,
            };

            // TRIGGER SYNTH VOICE (NOTE ON!)
            self.synth_voice_manager.borrow_mut().trigger_note(
                frequency,
                synth_waveform,
                adsr,
                gain_val,
                pan_val,
            );

            // Track latest event
            if event_start_abs > latest_triggered_start {
                latest_triggered_start = event_start_abs;
            }
        }
    }

    // Update last_trigger_time
    if latest_triggered_start > last_event_start {
        if let Some(Some(SignalNode::SynthPattern { last_trigger_time: lt, .. })) = self.nodes.get_mut(node_id.0) {
            *lt = latest_triggered_start as f32;
        }
    }

    // Output mixed audio from all synth voices
    self.synth_voice_manager.borrow_mut().process()
}
```

### Step 4: Add import statement

At the top of `src/unified_graph.rs` (around line 354):

```rust
use crate::sample_loader::SampleBank;
use crate::synth_voice_manager::SynthVoiceManager;
use crate::voice_manager::VoiceManager;
```

### Step 5: Update panic() method

In `panic()` method (around line 413):

```rust
pub fn panic(&mut self) {
    // Kill all active voices
    self.voice_manager.borrow_mut().kill_all();
    self.synth_voice_manager.borrow_mut().kill_all();

    // Hush all outputs
    self.hush_all();
}
```

### Step 6: Add DSL parser support

In `src/unified_graph_parser.rs`, add to `DslExpression` enum:

```rust
pub enum DslExpression {
    // ... existing variants

    SynthPattern {
        notes: String,          // Pattern of notes
        waveform: String,       // "sine", "saw", "square", "triangle"
        attack: Option<f32>,
        decay: Option<f32>,
        sustain: Option<f32>,
        release: Option<f32>,
        gain: Option<Box<DslExpression>>,
        pan: Option<Box<DslExpression>>,
    },
}
```

And add parsing for `synth("notes", waveform, params...)`.

### Step 7: Test it!

Create test file `test_synth_pattern.ph`:

```phonon
tempo: 0.5

# Pattern-triggered melody
~melody: synth("c4 e4 g4 c5", saw, attack=0.01, release=0.2)

out: ~melody * 0.3
```

Run:
```bash
cargo run --release --bin phonon -- render test_synth_pattern.ph output.wav --cycles 4
```

## Expected Result

- Melody plays 4 notes over 4 cycles
- Each note triggers a new synth voice
- ADSR envelope shapes each note
- Polyphonic (can play chords if overlapping)
- 64-voice polyphony

## Testing Checklist

- [ ] Audio is produced (not silence)
- [ ] Notes are distinct (not continuous drone)
- [ ] Frequencies are correct (c4 = 261.63 Hz)
- [ ] ADSR envelope shapes notes
- [ ] Multiple voices can overlap (polyphony)
- [ ] Voice stealing works at 65th voice
