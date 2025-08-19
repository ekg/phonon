# Phonon Development Roadmap

## Current Status ✅
- Basic pattern parsing (Strudel mini notation)
- Sample playback with lazy loading
- Real-time audio engine (cpal - JACK/ALSA/Android)
- OSC communication between Boson (JS) and Fermion (Rust)

## Phase 1: Core Pattern Language 🎵

### Pattern Operations
- [ ] **Concatenation** - `cat([pat1, pat2])` - play patterns in sequence
- [ ] **Stack** - `stack([pat1, pat2])` - play patterns simultaneously
- [ ] **Fast/Slow** - `fast(2)`, `slow(3)` - speed up/down patterns
- [ ] **Rev** - `rev()` - reverse pattern
- [ ] **Every** - `every(3, rev)` - apply function every n cycles
- [ ] **Euclidean rhythms** - `euclid(3,8)` - algorithmic patterns
- [ ] **Polyrhythm** - `{bd sn, hh hh hh}` - different lengths in parallel
- [ ] **Randomness** - `rand`, `irand`, `choose`, `chooseWith`
- [ ] **Conditional** - `when`, `whenmod` - conditional transformations

### Pattern Modifiers
- [ ] **Jux** - `jux(rev)` - stereo split with different effects
- [ ] **Chunk** - `chunk(4, fast(2))` - apply to chunks
- [ ] **Striate** - `striate(8)` - slice samples
- [ ] **Chop** - `chop(16)` - granular chopping
- [ ] **Splice** - `splice(8)` - fit sample to cycles
- [ ] **Legato** - `legato(0.5)` - note duration control
- [ ] **Nudge** - `nudge(0.1)` - time shifting

## Phase 2: Audio Effects & Modulation 🎛️

### Filters
- [ ] **Cutoff** - Low-pass filter frequency
- [ ] **Resonance** - Filter Q/resonance
- [ ] **Hcutoff/Hresonance** - High-pass filter
- [ ] **Bandf/Bandq** - Band-pass filter
- [ ] **Vowel** - Formant filter (a,e,i,o,u)
- [ ] **Filter envelope** - ADSR for filters

### Time Effects
- [ ] **Delay** - Echo effect with feedback
- [ ] **Room** - Reverb with size control
- [ ] **Leslie** - Rotating speaker effect
- [ ] **Orbit** - Separate effect buses

### Distortion & Modulation
- [ ] **Crush** - Bit crusher
- [ ] **Shape** - Waveshaping distortion
- [ ] **Distort** - General distortion
- [ ] **Phaser** - Phase shifting
- [ ] **Chorus** - Detuned copies
- [ ] **Tremolo** - Amplitude modulation
- [ ] **Vibrato** - Pitch modulation

### Dynamics
- [ ] **Compressor** - Dynamic range control
- [ ] **Limiter** - Peak limiting
- [ ] **Gate** - Noise gate
- [ ] **Duck** - Sidechain compression

## Phase 3: Synthesis 🎹

### Oscillators
- [ ] **Sine/Saw/Square/Triangle** - Basic waveforms
- [ ] **Noise** - White/pink/brown noise
- [ ] **FM synthesis** - Frequency modulation
- [ ] **AM synthesis** - Amplitude modulation
- [ ] **Additive** - Harmonic synthesis
- [ ] **Wavetable** - Table lookup synthesis

### Envelopes
- [ ] **ADSR** - Attack, Decay, Sustain, Release
- [ ] **Custom envelopes** - Arbitrary breakpoints
- [ ] **LFOs** - Low frequency oscillators
- [ ] **Envelope following** - Audio-triggered envelopes

## Phase 4: Advanced Pattern Features 🚀

### Scales & Harmony
- [ ] **Scale** - `scale("minor")` - quantize to scales
- [ ] **Chord** - `chord("Cm7")` - chord patterns
- [ ] **Arpeggio** - `arp("up")` - arpeggiation patterns
- [ ] **Voicing** - Smart voice leading
- [ ] **Transpose** - Pitch shifting

### Time & Tempo
- [ ] **Swing** - Groove/shuffle timing
- [ ] **Tempo changes** - Dynamic BPM
- [ ] **Polymetric** - Multiple simultaneous meters
- [ ] **Rubato** - Expressive timing variations

### Pattern Queries
- [ ] **Mask** - Boolean pattern operations
- [ ] **Struct** - Apply structure from one pattern to another
- [ ] **Inhabit** - Fill pattern with another

## Phase 5: Integration & Control 🔌

### MIDI
- [ ] **MIDI out** - Send notes/CC to hardware
- [ ] **MIDI in** - Control patterns with MIDI
- [ ] **Program changes** - Switch patches
- [ ] **Sysex** - System exclusive messages
- [ ] **Clock sync** - MIDI clock master/slave

### OSC & Network
- [ ] **OSC patterns** - Send arbitrary OSC
- [ ] **Network sync** - Collaborative live coding
- [ ] **Ableton Link** - Tempo sync protocol

### Visual
- [ ] **Hydra integration** - Visual patterns
- [ ] **Pattern visualization** - Real-time display
- [ ] **Scope/Spectrum** - Audio analysis display

## Phase 6: Performance Features 🎭

### Live Coding
- [ ] **Smooth transitions** - Pattern morphing
- [ ] **Cue system** - Prepare changes
- [ ] **History** - Undo/redo patterns
- [ ] **Presets** - Save/load patterns
- [ ] **Macros** - Custom shortcuts

### Recording
- [ ] **Audio recording** - Capture output
- [ ] **Pattern recording** - Save performances
- [ ] **Loop recording** - Overdub layers

## Implementation Strategy

### Rust (Fermion) Priorities:
1. **Audio effects pipeline** - Build modular effect chain
2. **Synthesis engine** - Oscillators and envelopes
3. **MIDI I/O** - Hardware integration
4. **Pattern scheduling** - Precise timing engine

### JavaScript (Boson) Priorities:
1. **Pattern parser expansion** - Full Strudel syntax
2. **Pattern operations** - Transformations and queries
3. **Scale/chord system** - Music theory helpers
4. **UI/visualization** - Web interface

### Architecture Improvements:
1. **Effect graph** - DAG-based audio routing
2. **Voice allocation** - Polyphonic voice management
3. **Buffer pool** - Zero-allocation audio processing
4. **Lock-free queues** - Thread-safe communication
5. **SIMD optimization** - Vectorized DSP

## Next Immediate Steps

1. **Add basic effects** (cutoff, resonance, delay, reverb)
2. **Implement pattern operations** (fast, slow, rev, every)
3. **Add synthesis** (basic oscillators with ADSR)
4. **Expand parser** (polyrhythm, euclidean patterns)
5. **MIDI output** (send to DAWs/hardware)

This roadmap would make Phonon a full-featured live coding environment comparable to TidalCycles/SuperCollider but with:
- Better Android/Termux support
- Native Rust performance
- No dependency on SuperCollider
- Direct hardware integration (JACK/MIDI)