# Tidal Sample Playback Architecture

## Key Findings from SuperDirt (Tidal's Audio Engine)

### Architecture Overview
Tidal uses a **voice-based architecture** where each event spawns an independent synth instance:

1. **DirtEvent.sc** (line 144-150): Each event calls `server.sendMsg(\s_new, ...)` which creates a **NEW synth instance** with auto-assigned node ID (`-1`)

2. **Independent Synth Groups** (line 176-177): Every event gets its own synth group:
   ```supercollider
   ~synthGroup = server.nextNodeID;
   server.sendMsg(\g_new, ~synthGroup, 1, outerGroup ? orbit.group);
   ```

3. **No Shared Playback State**: Each synth plays from sample start independently. Multiple "bd" events = multiple independent bd synth instances playing concurrently.

### How it Works
```
Tidal Pattern:   bd(3,8)
                   |
                   v
Query Events:    [Event{t=0.0, val="bd"}, Event{t=0.25, val="bd"}, Event{t=0.625, val="bd"}]
                   |
                   v
SuperDirt:       spawn_synth("bd", group_1) at t=0.0
                 spawn_synth("bd", group_2) at t=0.25
                 spawn_synth("bd", group_3) at t=0.625
                   |
                   v
Audio Output:    3 independent BD samples playing, potentially overlapping
```

## Our Current Bug

**Problem**: We use `HashMap<String, usize>` keyed by sample name only.
- All "bd" events share ONE playback cursor
- Cursor accumulates across events instead of resetting
- Causes timing drift

**Current Code** (`src/unified_graph.rs:544-558`):
```rust
let current_pos = positions.entry(sample_name.to_string()).or_insert(0);
// ^^^ All "bd" events share this cursor!
```

## Solution Options

### Option 1: Voice Pool (SuperDirt-style) ✅ RECOMMENDED
```rust
struct Voice {
    sample_name: String,
    sample_data: Arc<Vec<f32>>,
    playback_position: usize,
    active: bool,
}

struct SamplePlayback {
    voices: Vec<Voice>,
    max_voices: usize,
}

impl SamplePlayback {
    fn trigger(&mut self, sample_name: &str) -> &mut Voice {
        // Find inactive voice or steal oldest
        let voice = self.find_free_voice();
        voice.sample_name = sample_name.to_string();
        voice.playback_position = 0; // Reset!
        voice.active = true;
        voice
    }
    
    fn render_sample(&mut self) -> f32 {
        let mut output = 0.0;
        for voice in &mut self.voices {
            if voice.active {
                output += voice.sample_data[voice.playback_position];
                voice.playback_position += 1;
                if voice.playback_position >= voice.sample_data.len() {
                    voice.active = false; // Finished
                }
            }
        }
        output
    }
}
```

### Option 2: Event-Instance Keyed HashMap
```rust
// Track positions by (sample_name, event_hash)
HashMap<(String, u64), usize>

// Problem: Need to compute event hash from Event struct
// Also need to clean up old entries
```

## Recommendation
Implement **Option 1: Voice Pool** because:
1. ✅ Matches SuperDirt/Tidal architecture exactly
2. ✅ Natural support for polyphony (multiple overlapping samples)
3. ✅ Efficient memory usage with voice stealing
4. ✅ Clean separation of concerns
5. ✅ Easy to add per-voice effects later (pitch, filter, etc.)

Voice pool size: 64-128 voices should be plenty for live coding.
