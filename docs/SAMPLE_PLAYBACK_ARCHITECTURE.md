# Sample Playback Architecture Research Report

## Current Problem

Our current implementation using fundsp's `envelope` function has a critical flaw: it doesn't properly sequence through samples. The envelope uses time as a direct index into the sample buffer, which causes:
- Samples that start with silence (like bd) don't produce any output
- No proper sample position tracking
- Cannot handle overlapping samples properly

## How Other Systems Handle Sample Playback

### SuperCollider/SuperDirt Architecture

SuperCollider uses a sophisticated client-server architecture:

1. **Buffer System**: 
   - Samples are loaded into server-side buffers (globally indexed arrays of 32-bit floats)
   - Buffers can be allocated/freed while synthesis is running
   - Each buffer has a unique ID number for reference

2. **Synth Architecture**:
   - Each sample trigger creates a new Synth instance
   - Synths use `PlayBuf` UGen to play buffer contents
   - `doneAction: 2` automatically frees synth memory after playback
   - Multiple synths can play the same buffer simultaneously

3. **Polyphony**:
   - Unlimited polyphony through dynamic synth allocation
   - Each trigger creates a new voice
   - Automatic resource management prevents memory leaks

### TidalCycles Approach

TidalCycles separates concerns:
- **Pattern Generation**: Tidal only generates patterns and timing events
- **Sound Generation**: Delegates to SuperDirt via OSC messages
- **Sample Triggering**: Each pattern event sends a trigger message with parameters
- **Polyphony**: Handled entirely by SuperDirt's synth allocation

### Rust Audio Libraries Comparison

#### Rodio (Built on CPAL)
- **Pros**: Unlimited simultaneous sounds with automatic mixing
- **Cons**: Higher-level abstraction, less control
- **Polyphony**: Excellent - spawns background thread for mixing

#### Kira
- **Pros**: Sophisticated mixing, effects, precise timing
- **Cons**: More complex, game-oriented
- **Polyphony**: Excellent with built-in mixer

#### Glicol
- **Pros**: Graph-based, live coding oriented, WebAssembly support
- **Cons**: Still experimental
- **Polyphony**: Handles through graph nodes

## Proposed Solution for Phonon

### Architecture Design

```
┌─────────────────────────────────────────────────────┐
│                  Pattern Sequencer                  │
│  - Tracks global time                               │
│  - Queries patterns for events                      │
│  - Triggers voices at precise times                 │
└──────────────────────┬──────────────────────────────┘
                       │ Trigger events
                       ▼
┌─────────────────────────────────────────────────────┐
│                   Voice Manager                     │
│  - Allocates new voices for each trigger            │
│  - Manages voice pool (reuse finished voices)       │
│  - Handles voice stealing if needed                 │
└──────────────────────┬──────────────────────────────┘
                       │ Create/manage
                       ▼
┌─────────────────────────────────────────────────────┐
│                   Sample Voices                     │
│  - Independent playback position                    │
│  - Own envelope state                               │
│  - Can be pitched/filtered/effected                 │
└──────────────────────┬──────────────────────────────┘
                       │ Audio output
                       ▼
┌─────────────────────────────────────────────────────┐
│                      Mixer                          │
│  - Sums all active voices                          │
│  - Applies global effects                          │
│  - Outputs to audio device                         │
└─────────────────────────────────────────────────────┘
```

### Implementation Strategy

#### Option 1: Pure FunDSP Approach (Limited)
- Use `Net` for dynamic graph building
- Create multiple `wavech` nodes for polyphony
- Problem: fundsp's Wave playback is stateless/envelope-based

#### Option 2: Custom Voice System (Recommended)
- Build our own voice allocation system
- Each voice maintains its own playback position
- Mix voices manually before sending to fundsp for effects

#### Option 3: Use External Library for Playback
- Use Rodio or Kira for sample playback
- Use fundsp for effects processing
- Problem: Integration complexity

### Immediate Fix Implementation

1. **Create Stateful Sample Player**:
   - Track position for each playing sample
   - Output samples sequentially
   - Handle sample completion

2. **Voice Pool Management**:
   - Pre-allocate voice pool (e.g., 32 voices)
   - Mark voices as active/inactive
   - Reuse inactive voices for new triggers

3. **Mixing Strategy**:
   - Sum all active voice outputs
   - Apply per-voice gain/pan before mixing
   - Send mixed output through fundsp effects chain

## Implementation Plan

### Phase 1: Basic Stateful Playback
- Replace envelope-based player with position-tracking player
- Fix bd/sn playback issue
- Support overlapping samples

### Phase 2: Voice Management
- Implement voice pool
- Add voice stealing for polyphony limit
- Optimize performance

### Phase 3: Advanced Features
- Per-voice effects (pitch, filter)
- Sample start/end positions
- Loop points
- Crossfading for voice stealing

## Code Architecture

```rust
// Core structures needed:

struct Voice {
    sample_data: Arc<Vec<f32>>,
    position: usize,
    active: bool,
    gain: f32,
}

struct VoiceManager {
    voices: Vec<Voice>,
    next_voice: usize,
}

struct SampleMixer {
    voice_manager: VoiceManager,
    output_buffer: Vec<f32>,
}

impl SampleMixer {
    fn trigger_sample(&mut self, sample: Arc<Vec<f32>>) {
        // Find inactive voice or steal oldest
        let voice = self.voice_manager.allocate_voice();
        voice.start_playback(sample);
    }
    
    fn process_block(&mut self, size: usize) -> Vec<f32> {
        // Mix all active voices
        for voice in &mut self.voice_manager.voices {
            if voice.active {
                voice.process_into(&mut self.output_buffer);
            }
        }
        self.output_buffer.clone()
    }
}
```

## Conclusion

The current envelope-based approach is fundamentally flawed for proper sample playback. We need a stateful, voice-based system similar to SuperCollider's architecture but implemented in Rust. This will provide:

1. Proper sample playback from start to finish
2. Unlimited overlapping samples
3. Per-voice control (pitch, gain, effects)
4. Efficient resource management
5. Compatibility with pattern sequencing

The recommended approach is to build a custom voice management system that handles sample playback independently of fundsp, then use fundsp for effects processing on the mixed output.