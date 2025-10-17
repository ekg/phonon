# Unified Signal Graph Architecture for Phonon

## Current State Analysis

### What We Have (Fragmented)
1. **SimpleDspExecutor** - Basic audio generation
2. **SimpleDspExecutorV2** - Pattern parameters support
3. **SignalGraph** - Graph structure but not fully integrated
4. **Pattern System** - Working but separate from synthesis
5. **glicol_dsp/glicol_dsp_v2** - DSP nodes but not unified

### The Problem
- Multiple executors doing similar things
- Patterns and synthesis are separate worlds
- No real modulation routing between arbitrary signals
- Can't define custom synths inline with patterns
- Signal analysis exists but isn't integrated

## The Vision: Everything Is A Signal

### Core Principles
1. **Unified Graph** - One signal graph rules them all
2. **Patterns as Signals** - Pattern events are just another type of signal
3. **Universal Modulation** - Any signal can modulate any parameter
4. **Live Synthesis** - Define synths inline, modify on the fly
5. **No Black Boxes** - Every connection is visible and routable

## Architecture Design

### 1. Core Signal Graph Engine

```rust
pub struct UnifiedSignalGraph {
    // All nodes in the graph
    nodes: HashMap<NodeId, SignalNode>,

    // Named buses for referencing
    buses: HashMap<String, NodeId>,

    // Connections between nodes
    connections: Vec<Connection>,

    // Pattern state (integrated!)
    pattern_state: PatternEngine,

    // Sample rate and timing
    sample_rate: f32,
    cycle_position: f64,
    cps: f32,
}

pub enum SignalNode {
    // Sources
    Oscillator { freq: Signal, waveform: WaveType },
    Pattern { pattern: String, evaluated: Vec<Event> },
    Constant { value: f32 },

    // Processors
    Filter { input: Signal, cutoff: Signal, q: Signal },
    Envelope { input: Signal, adsr: ADSR },
    Delay { input: Signal, time: Signal, feedback: Signal },

    // Analysis
    RMS { input: Signal, window: f32 },
    Pitch { input: Signal },
    Transient { input: Signal },

    // Math
    Add { a: Signal, b: Signal },
    Multiply { a: Signal, b: Signal },

    // Control
    When { input: Signal, condition: Signal },
    Route { input: Signal, destinations: Vec<(NodeId, f32)> },
}

pub enum Signal {
    Node(NodeId),           // Reference to another node
    Bus(String),           // Reference to named bus
    Pattern(String),       // Inline pattern
    Value(f32),           // Constant value
    Expression(Box<Expr>), // Math expression
}
```

### 2. Pattern Integration

Patterns become first-class signal nodes:

```
// Pattern as modulation source
~cutoff_pattern: "1000 2000 500 3000"
~bass: saw(110) # lpf(~cutoff_pattern, 0.8)

// Pattern triggering synthesis
~kick_pattern: "bd ~ ~ bd"
~kick: sine(60) * perc(0.01, 0.2) * ~kick_pattern

// Audio modulating pattern playback
~gate: ~input # rms(0.05) # thresh(0.1)
~drums: "bd sn hh cp" # when(~gate)
```

### 3. Inline Synth Definitions

Define synthesizers as part of the pattern language:

```
// Define reusable synth
synthdef ~acid_bass($note, $gate): {
    ~osc: saw($note # mtof) + square($note # mtof - 0.05)
    ~env: adsr($gate, 0.01, 0.2, 0.3, 0.5)
    ~filter: ~osc # lpf(~env * 4000 + 200, 0.9)
    out: ~filter * ~env * 0.3
}

// Use in pattern with note data
~bass_line: "c2 eb2 g2 c3" # ~acid_bass

// Modulate synth params with patterns
~filter_mod: "1 0.5 0.8 0.3"
~bass_line: "c2 eb2 g2 c3" # ~acid_bass[filter: ~filter_mod]
```

### 4. Universal Modulation Matrix

Any signal can modulate any parameter:

```
// LFO modulating multiple targets
~lfo: sine(2) * 0.5 + 0.5
route ~lfo -> {
    ~bass.filter.cutoff: 2000,    // Modulate by ±2000 Hz
    ~lead.pan: 0.8,               // Pan modulation
    ~drums.speed: 0.2             // Pattern speed modulation
}

// Audio-rate modulation (FM synthesis)
~carrier: sine(440)
~modulator: sine(110) * 200
~fm_out: sine(440 + ~modulator)

// Pattern modulating audio
~rhythm: "1 0 1 0" * 0.5
~gated: ~synth * ~rhythm
```

### 5. Implementation Phases

#### Phase 1: Unified Graph Foundation ✅ (What we need NOW)
- [ ] Create `UnifiedSignalGraph` that replaces all executors
- [ ] Migrate pattern evaluation into the graph
- [ ] Implement bus system with `~name` references
- [ ] Basic signal routing with `#`

#### Phase 2: Pattern-Audio Bridge
- [ ] Patterns as signal sources
- [ ] Pattern triggers for envelopes
- [ ] Audio analysis nodes (RMS, pitch, etc.)
- [ ] Conditional routing (`when`, `if`)

#### Phase 3: Modulation Routing
- [ ] `route` statement for explicit modulation
- [ ] Multi-destination routing
- [ ] Modulation scaling and mapping
- [ ] Feedback loops with implicit delay

#### Phase 4: Inline Synthesis
- [ ] `synthdef` definitions
- [ ] Voice allocation for polyphony
- [ ] Parameter passing to synths
- [ ] Pattern-driven synthesis

#### Phase 5: Advanced Features
- [ ] Probability-based routing
- [ ] Complex feedback networks
- [ ] Sample-accurate timing
- [ ] Pattern algebra integration

## Why This Matters

### What This Enables
1. **Live Patching**: Rewire your entire setup while performing
2. **Deep Integration**: Patterns and audio are one system
3. **Creative Freedom**: Any signal can control anything
4. **Performance**: One unified graph = better optimization
5. **Clarity**: One mental model instead of multiple systems

### Example: The Power of Unification

```
// Old way (current):
// - Pattern in one system
// - Synth in another
// - No cross-modulation

// New way:
~kick: "bd ~ ~ bd"
~bass: "c2 eb2 g2 bb2"
~bass_env: ~kick # inv # lag(0.01)  // Sidechain from kick pattern!
~bass_synth: saw(~bass # mtof) * ~bass_env # lpf(~kick * 2000 + 500)

// The kick PATTERN modulates the bass filter!
// The bass amplitude is ducked by the kick!
// Everything is connected!
```

## Next Immediate Steps

1. **Consolidate Executors**: Merge SimpleDspExecutor and V2 into UnifiedSignalGraph
2. **Pattern Integration**: Make patterns evaluate inside the graph
3. **Bus System**: Implement the `~name` reference system properly
4. **Signal Analysis**: Add RMS, pitch detection as graph nodes
5. **Routing**: Implement the `#` operator for signal flow

This is the path to making Phonon a true modular synthesis live coding environment where **everything is connected** and **nothing is hidden**.