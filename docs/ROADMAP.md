# Phonon Development Roadmap

**Last Updated**: 2025-11-25
**Current Status**: Core vision complete, expanding synthesis capabilities

---

## Current State

### Test Suite
- **1836+ tests passing**
- **137 node types implemented**
- Comprehensive coverage for patterns, synthesis, effects

### What's Working

#### DSL Syntax (Complete)
```phonon
cps: 2.0

-- Audio buses with $ (signal generators)
~drums $ s "bd sn hh*4 cp"
~bass $ saw "55 82.5"

-- Modifier buses with # (parameter control)
~lfo # sine 2
~cutoff # "500 1000 2000"

-- Effect chaining
~filtered $ saw 55 # lpf (~lfo * 500 + 800) 0.8

-- Pattern transforms
~fast $ s "bd sn" $ fast 2 $ rev

-- Sample bank selection
~kicks $ s "bd:0 bd:1 bd:2"

-- Output
out $ ~drums * 0.5 + ~bass * 0.3
```

#### Core Features
- Voice-based sample playback (64 voices, polyphonic)
- Pattern-controlled synthesis at sample rate (44.1kHz)
- Mini-notation: Euclidean rhythms, alternation, subdivision, rests
- Live coding with auto-reload (`phonon live`)
- Multi-output system (`out1`, `out2`, etc.)
- Hush/Panic commands

#### Pattern Transformations
- `fast`, `slow`, `rev`, `every`
- `jux`, `stutter`, `chop`, `degradeBy`, `scramble`
- `rotL`, `rotR`, `iter`, `palindrome`
- Bidirectional operators: `$`, `#`, `|>`, `<|`

#### Synthesis (137 nodes)
**Oscillators**: sine, saw, square, triangle, pulse, polyblep, wavetable, FM, PM, granular, noise (white, pink, brown), blip, impulse

**Filters**: lpf, hpf, bpf, notch, moog ladder, SVF, comb, allpass, resonz, formant, DJ filter, parametric EQ

**Envelopes**: ADSR, AR, ASR, AD, line, xline, segments, curve

**Effects**: reverb, delay, distortion, compressor, limiter, bitcrush, chorus, flanger, phaser, tremolo, vibrato, ring mod, pitch shifter, vocoder, frequency shifter, stereo widener, tape delay, convolution

**Dynamics**: compressor, expander, limiter, gate, noise gate, sidechain compressor, envelope follower, transient shaper

**Utilities**: gain, pan, mix, clip, fold, wrap, quantize, sample & hold, lag, slew limiter, crossfade

#### DSP Parameters (per-voice)
- `gain`, `pan`, `speed`, `cut`, `attack`, `release`
- All pattern-controllable

---

## What's Next

### Short Term

1. **Fix Failing Tests** (2 tests)
   - `test_very_efficient_cpu` (one_pole_filter)
   - `test_allpass_flat_magnitude_response`

2. **Documentation Cleanup**
   - Update syntax examples to use new `$` and `#` operators
   - Remove outdated references
   - Add cookbook of common patterns

### Medium Term

1. **DAW-Style Buffer Architecture** (performance optimization)
   - Block-based buffer passing instead of sample-by-sample
   - Parallel node execution for independent chains
   - Currently graph traversed 512x per block

2. **MIDI Output** (when needed)
   - Send patterns as MIDI notes
   - Hardware integration

3. **OSC Integration** (when needed)
   - Network communication with other software

---

## What Makes Phonon Unique

**Tidal/Strudel** (Event-based):
```haskell
d1 $ sound "bd sn"  -- Triggers discrete events
```

**Phonon** (Signal-based):
```phonon
~lfo # sine 0.25
out $ saw 55 # lpf (~lfo * 2000 + 500) 0.8
-- Pattern modulates filter continuously at sample rate!
```

In Tidal/Strudel, patterns only trigger discrete events. In Phonon, patterns ARE control signals evaluated at 44.1kHz, enabling real-time modulation of any synthesis parameter.

---

## Design Principles

1. **Every parameter is a pattern** - No bare types, all `Pattern<T>`
2. **Space-separated syntax** - `lpf 1000 0.8` not `lpf(1000, 0.8)`
3. **Test-driven development** - Write failing test first
4. **Three-level audio testing** - Pattern query, onset detection, RMS

---

## Architecture

**Parser**: `src/unified_graph_parser.rs`
**Compiler**: `src/compositional_compiler.rs`
**Audio Engine**: `src/unified_graph.rs`
**Nodes**: `src/nodes/*.rs` (137 files)
**Patterns**: `src/pattern.rs`, `src/pattern_ops.rs`
**Mini-notation**: `src/mini_notation_v3.rs`
