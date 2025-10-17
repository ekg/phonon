# Pattern Flow Analysis - How Patterns Trigger Voices

## The Flow (Sample Playback)

```
Pattern String ("bd sn bd sn")
    ↓
parse_mini_notation()
    ↓
Pattern<String> object
    ↓
SignalNode::Sample { pattern, last_trigger_time, ... }
    ↓
EVERY SAMPLE (44100/sec):
    pattern.query(cycle_position) → Returns Events
    ↓
Events have:
  - event.value (sample name: "bd", "sn", etc.)
  - event.whole.begin (start time in cycles)
  - event.part (active time span)
    ↓
For each event:
  if event_start > last_trigger_time:
    voice_manager.trigger_sample(sample_data, gain, pan, speed)
    last_trigger_time = event_start
    ↓
voice_manager.process() → Mixed audio from all 64 voices
```

## Key Code (unified_graph.rs lines 1290-1366)

### 1. Query Pattern Every Sample
```rust
let sample_width = 1.0 / self.sample_rate as f64 / self.cps as f64;
let state = State {
    span: TimeSpan::new(
        Fraction::from_float(self.cycle_position),
        Fraction::from_float(self.cycle_position + sample_width),
    ),
    controls: HashMap::new(),
};
let events = pattern.query(&state);
```

**This is the magic**: Pattern is queried at the current cycle position, returns **Events** (discrete triggers).

### 2. Detect New Events (Note On)
```rust
for event in events.iter() {
    let sample_name = event.value.trim();

    // Skip rests (~)
    if sample_name == "~" || sample_name.is_empty() {
        continue;
    }

    // Get event start time
    let event_start_abs = if let Some(whole) = &event.whole {
        whole.begin.to_float()
    } else {
        event.part.begin.to_float()
    };

    // Only trigger NEW events (not already triggered)
    let event_is_new = event_start_abs > last_event_start + tolerance;

    if event_is_new {
        // TRIGGER VOICE (NOTE ON!)
        self.voice_manager.borrow_mut().trigger_sample_with_cut_group(
            sample_data,
            gain_val,
            pan_val,
            speed_val,
            cut_group_opt,
        );

        latest_triggered_start = event_start_abs;
    }
}

// Update last_trigger_time so we don't re-trigger
if latest_triggered_start > last_event_start {
    last_trigger_time = latest_triggered_start;
}
```

**This is NOTE ON**: When an event starts that we haven't triggered yet, trigger a voice.

### 3. Output Voice Audio
```rust
// After triggering, output mixed audio from all voices
self.voice_manager.borrow_mut().process()
```

## The Pattern System Already Provides Note On/Off!

**Events have time spans**:
```rust
Event {
    value: "bd",           // What to trigger
    whole: (0.0, 0.25),   // Conceptual time (start, end)
    part: (0.0, 0.25),    // Actual sounding time
}
```

- `whole.begin` = **NOTE ON time**
- `whole.end` = **NOTE OFF time** (implicit)

The pattern system **already has** note on/off timing! We just need to use it for synths.

## What's Missing for Synths

Samples work because:
1. ✅ Pattern queries return Events
2. ✅ Events trigger voices (VoiceManager)
3. ✅ Voices play pre-recorded audio
4. ✅ Voices stop when sample ends

Synths DON'T work because:
1. ✅ Pattern queries return Events (same)
2. ❌ No synth voice spawning
3. ❌ No frequency from pattern (event.value should be "c4", "a3", "g3")
4. ❌ No ADSR envelope per voice
5. ❌ No voice stopping (synths are continuous)

## What We Need: SynthVoiceManager

```rust
// Like VoiceManager but for synths
pub struct SynthVoiceManager {
    voices: [Option<SynthVoice>; 64],  // 64 polyphonic voices
    next_voice: usize,
}

pub struct SynthVoice {
    oscillator: Oscillator,
    envelope: EnvelopeState,
    frequency: f32,
    gain: f32,
    pan: f32,
    is_active: bool,
}

impl SynthVoiceManager {
    // NOTE ON
    fn trigger_note(&mut self, frequency: f32, gain: f32, envelope_params: ADSR) {
        // Find free voice or steal oldest
        let voice = self.get_free_voice();
        voice.frequency = frequency;
        voice.envelope.trigger();  // Start ADSR
        voice.is_active = true;
    }

    // NOTE OFF (could be from pattern end or explicit)
    fn release_note(&mut self, voice_id: usize) {
        self.voices[voice_id].envelope.release();
    }

    // Process all voices
    fn process(&mut self) -> f32 {
        let mut mix = 0.0;
        for voice in &mut self.voices {
            if let Some(v) = voice {
                if v.is_active {
                    let osc_sample = v.oscillator.process(v.frequency);
                    let env_level = v.envelope.process();
                    mix += osc_sample * env_level * v.gain;

                    // Stop voice when envelope finishes
                    if env_level == 0.0 && v.envelope.is_released() {
                        v.is_active = false;
                    }
                }
            }
        }
        mix.tanh()  // Soft clip
    }
}
```

## SignalNode::SynthPattern

```rust
SignalNode::SynthPattern {
    note_pattern: Pattern<String>,       // Pattern of notes: "c4 a3 g3 e4"
    oscillator_type: Waveform,           // Sine, Saw, Square
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    gain: Signal,                        // Can be modulated
    pan: Signal,
    last_trigger_time: f32,
}
```

## DSL Syntax

```phonon
tempo: 2.0

# Pattern-triggered synth
~melody: synth("c4 e4 g4 c5", saw, attack=0.01, release=0.2) * 0.3

# Pattern-triggered bass
~bass: synth("c2 ~ g2 ~", square, attack=0.05, release=0.3) * 0.5

out: ~melody + ~bass
```

## Implementation Plan

1. **Create `SynthVoiceManager`** (like `VoiceManager` but for oscillators)
   - 64 polyphonic voices
   - Each voice has: oscillator + ADSR + frequency
   - `trigger_note(freq)` = NOTE ON
   - `release_note(voice_id)` = NOTE OFF
   - `process()` = mix all active voices

2. **Add `SignalNode::SynthPattern`**
   - Query pattern for events (like Sample node does)
   - Parse event.value as note name ("c4" → 261.63 Hz)
   - Trigger synth voice on new events
   - Output mixed audio from all synth voices

3. **Note Length**
   - Use `event.whole.end - event.whole.begin` for note duration
   - After duration, call `envelope.release()`
   - Voice stays active until envelope fully decays

4. **DSL Parser Support**
   - Add `synth(pattern, waveform, adsr_params)` function
   - Compile to `SignalNode::SynthPattern`

## Why This Works

The pattern system is **already designed for this**:
- Patterns return discrete Events with timing
- Events have start/end times (note on/off)
- Sample playback already uses this correctly
- We just need to apply the SAME logic to synths

**Samples**: Event triggers pre-recorded audio
**Synths**: Event triggers oscillator + ADSR

Same architecture, different audio source!
