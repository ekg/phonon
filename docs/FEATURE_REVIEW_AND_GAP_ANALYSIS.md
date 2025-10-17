# Phonon Feature Review & Gap Analysis

**Date**: 2025-10-14
**Purpose**: Comprehensive review of what's implemented vs what's missing

## Executive Summary

Phonon is a **hybrid live coding system** with a unique architecture:
- Patterns are control signals (sample-rate evaluation)
- Everything flows through one unified signal graph
- Sample-based playback with 64-voice polyphony
- Real-time synthesis with pattern modulation

**Current Status**: Core architecture solid, missing some high-level features

---

## ✅ What's Fully Implemented and Working

### 1. Core Audio Engine
- **Unified Signal Graph**: Sample-by-sample processing at 44.1kHz
- **Voice Manager**: 64-voice polyphonic sample playback
- **Sample Bank**: Loads samples from `samples/` directory (dirt-samples compatible)
- **Multi-output**: Supports multiple output channels (`out1`, `out2`, etc.)
- **Real-time Processing**: Low-latency audio output

### 2. Pattern System (Mini-Notation)
- **Basic Patterns**: `"bd sn cp hh"`
- **Multiplication**: `"bd*4"`
- **Rests**: `"bd ~ sn ~"`
- **Grouping**: `"[bd sn] hh"`
- **Alternation**: `"bd <sn cp>"` - cycles between options
- **Euclidean Rhythms**: `"bd(3,8)"` - 3 hits in 8 steps
- **Sample Selection**: `"bd:0 bd:1 bd:2"` - numbered sample banks

### 3. Synthesis (SuperDirt-Inspired Synths)
All implemented in `src/superdirt_synths.rs`:

**Drums**:
- ✅ `superkick(freq, pitch_env, sustain, noise)` - Kick with pitch envelope
- ✅ `supersnare(freq, snappy, sustain)` - Snare with noise layer
- ✅ `superhat(bright, sustain)` - Hi-hat with filtered noise

**Melodic**:
- ✅ `supersaw(freq, detune, voices)` - **DETUNE IS IMPLEMENTED**
- ✅ `superpwm(freq, pwm_rate, pwm_depth)` - Pulse width modulation
- ✅ `superchip(freq, vibrato_rate, vibrato_depth)` - Chiptune square wave
- ✅ `superfm(freq, mod_ratio, mod_index)` - 2-operator FM

**Basic Oscillators**:
- ✅ `sine(freq)`, `saw(freq)`, `square(freq)`, `triangle(freq)`
- ✅ `noise` - White noise generator

### 4. Effects
- ✅ `lpf(cutoff, q)` - Low-pass filter (State Variable Filter)
- ✅ `hpf(cutoff, q)` - High-pass filter
- ✅ `reverb(room_size, damping, mix)` - Freeverb algorithm
- ✅ `distortion(drive, mix)` - Soft clipping waveshaper
- ✅ `bitcrush(bits, sample_rate)` - Bitcrusher
- ✅ `chorus(rate, depth, mix)` - Chorus effect

### 5. Pattern-to-Synthesis Integration (UNIQUE FEATURE!)
- ✅ **Pattern-controlled oscillator frequency**: `sine("110 220 440")`
- ✅ **Pattern-controlled filter cutoff**: `saw(55) # lpf("500 2000", 0.8)`
- ✅ **Patterns as control signals**: Any parameter can be pattern-modulated
- ✅ **Pattern value holding**: Values persist between triggers

This is **Phonon's killer feature** - Tidal can't do this!

### 6. Live Coding Environment
- ✅ `phonon live x.ph` - File watching with auto-reload
- ✅ Real-time audio output
- ✅ Error messages displayed in terminal

### 7. DSL Parser
- ✅ Bus assignment: `~name = expression`
- ✅ Output assignment: `out = expression` or `out1 = expression`
- ✅ Tempo setting: `cps: 2.0`
- ✅ Signal chaining: `saw(110) # lpf(1000, 0.8) # distortion(5.0, 0.3)`
- ✅ Arithmetic: `~a + ~b`, `~osc * 0.5`, `~lfo * 1000 + 500`
- ✅ Sample triggering: `s("bd sn")`
- ✅ Synth calls: `supersaw(220, 0.5, 5)`

---

## ❌ What's Missing or Broken

### 1. Pattern Transformations (⚠️ PARTIALLY IMPLEMENTED)
Status varies by implementation:
- ⚠️ `|> fast(n)` - ✅ CLI works, ❌ DslCompiler broken
- ⚠️ `|> slow(n)` - ✅ CLI works, ❌ DslCompiler broken
- ⚠️ `|> rev` - ✅ CLI parses but produces silence (BUG), ❌ DslCompiler broken
- ⚠️ `|> every(n, fn)` - ✅ CLI works, ❌ DslCompiler broken
- ❌ `|> rotate(n)` - Not implemented anywhere

**Status**: Core Pattern methods exist and work. CLI has custom `$` parsing (main.rs lines 618-697). DslCompiler has NO `$` support.
**Details**: See `docs/PATTERN_TRANSFORMS_STATUS.md`

### 2. Pattern Frequency Parameters (✅ FIXED)
- ✅ Pattern strings for frequency now work correctly for synths
- **Fixed**: `sine("110 220 330")` now correctly cycles through 110, 220, 330 Hz
- **Fixed**: `supersaw("110 220", 0.5, 5)` correctly cycles frequencies

**Root Cause**: Wrong evaluation order in `unified_graph.rs:915-927` - tried MIDI note parsing before numeric parsing
**Fix Date**: 2025-10-14
**Details**: See `docs/PATTERN_FREQUENCY_BUG_FIX.md`

### 3. DSP Parameter Patterns (Tidal-style)
Not implemented:
- ❌ `s("bd sn", gain="0.8 1.0")` - Per-event gain
- ❌ `s("bd sn", pan="0 1")` - Per-event pan
- ❌ `s("bd sn", speed="1 0.5")` - Per-event speed
- ❌ `s("bd sn", cut="1")` - Cut groups

**Note**: These would need to be implemented as per-voice modulation

### 4. Advanced Pattern Features
Tidal features not in Phonon:
- ❌ Pattern operations: `+|`, `*|`, etc. (pattern as function)
- ❌ `stack` - Layer multiple patterns
- ❌ `cat` - Concatenate patterns
- ❌ Conditional operators: `when`, `while`
- ❌ Random selection: `choose`, `irand`
- ❌ Probability: `?` operator

### 5. Control Features
- ❌ `hush` command (implemented but not tested)
- ❌ `panic` command (implemented but not tested)
- ❌ MIDI clock sync
- ❌ OSC control (partially implemented?)

---

## ⚠️ What's Implemented But Needs Better Testing

### 1. Detune Parameter
- **Status**: IMPLEMENTED (verified in `superdirt_synths.rs:168-210`)
- **Problem**: Test used wrong FFT analysis method
- **Fix Needed**: Proper FFT test that analyzes fundamental frequency distribution, not total bandwidth

### 2. Multi-Output System
- **Status**: Implemented in `unified_graph.rs`
- **Needs**: Integration tests

### 3. Polyphony
- **Status**: 64-voice manager exists
- **Needs**: Tests for voice stealing, overlapping samples

---

## Comparison: Phonon vs Tidal vs Glicol

### Pattern Syntax

| Feature | Tidal | Glicol | Phonon | Status |
|---------|-------|--------|--------|--------|
| Basic patterns | `"bd sn"` | `seq "bd sn"` | `"bd sn"` | ✅ |
| Multiplication | `"bd*4"` | - | `"bd*4"` | ✅ |
| Euclidean | `"bd(3,8)"` | - | `"bd(3,8)"` | ✅ |
| Alternation | `"<bd sn>"` | - | `"<bd sn>"` | ✅ |
| Transformations | `fast 2` | - | `\|> fast(2)` | ❌ Not implemented |
| Layering | `stack [...]` | - | - | ❌ |

### Synthesis

| Feature | Tidal (via SuperDirt) | Glicol | Phonon | Status |
|---------|----------------------|--------|--------|--------|
| Basic oscillators | ✅ | ✅ | ✅ | ✅ |
| Filters | ✅ | ✅ | ✅ | ✅ |
| SuperSaw | ✅ | ❌ | ✅ | ✅ |
| FM Synthesis | ✅ | ✅ | ✅ | ✅ |
| Pattern-modulated params | ❌ | Partial | ✅ | ✅ **UNIQUE!** |

### Architecture

| Aspect | Tidal | Glicol | Phonon |
|--------|-------|--------|--------|
| Language | Haskell | JavaScript | Rust |
| Pattern Engine | Event-based | Event-based | **Signal-based** (unique!) |
| Audio Engine | SuperCollider | WebAudio API | Native Rust |
| Update Rate | Per-cycle | Per-cycle | **Per-sample** (unique!) |
| Can patterns modulate synthesis? | No (discrete events) | Partial | **Yes** (continuous control!) |

---

## Phonon's Unique Selling Points

### 1. Patterns ARE Control Signals
```phonon
~lfo = "0.5 1.0"  # Pattern as control signal
out = sine(220) * ~lfo  # Continuous amplitude modulation
```
Tidal can't do this - patterns trigger discrete events, can't continuously modulate.

### 2. Everything in One Graph
```phonon
out = s("bd sn") # lpf(2000, 0.8) # reverb(0.7, 0.5, 0.3)
```
Samples flow through effects just like synthesis. No separation between pattern engine and audio engine.

### 3. Sample-Accurate Timing
All evaluation happens at 44.1kHz, not just at cycle boundaries.

### 4. Rust Performance
Native code, no interpreter overhead, suitable for real-time audio.

---

## Priority Fixes

### HIGH PRIORITY (This Week)

1. ✅ **Fix Pattern Frequency Parameters** - COMPLETED 2025-10-14
   - Fixed wrong evaluation order in `unified_graph.rs:915-927`
   - Created comprehensive FFT diagnostic tests
   - All tests passing

2. ✅ **Implement Proper Detune Test** - COMPLETED 2025-10-14
   - Created FFT test analyzing fundamental frequency distribution
   - Verified detune parameter works correctly
   - Test measures frequency peak spacing

3. **Pattern Transformations (Just 3-5 essential ones)** - TODO
   - `fast(n)` - Most important
   - `slow(n)` - Second most important
   - `rev` - Nice to have
   - Don't implement all of Tidal, just the basics

### MEDIUM PRIORITY (Next Sprint)

4. **DSP Parameter Patterns**
   - `s("bd", gain="0.8 1.0")`
   - `s("bd", pan="-1 1")`
   - `s("bd", speed="1 0.5")`

5. **Test Multi-Output**
   - Verify `out1`, `out2` work
   - Test `hush` command
   - Test `panic` command

6. **Polyphony Testing**
   - Verify 64 voices
   - Test voice stealing
   - Test overlapping samples

### LOW PRIORITY (Future)

7. **Advanced Pattern Features**
   - Pattern layering
   - Random/probabilistic patterns
   - Pattern conditionals

8. **Documentation**
   - Update PHONON_LANGUAGE_REFERENCE.md
   - Create comparison to Tidal
   - Add more examples

---

## Testing Gaps

### 1. Detune (✅ FIXED)
**Was**: Measures total bandwidth (includes harmonics)
**Now**: ✅ Measures fundamental frequency peak spacing using FFT
**Test**: `test_supersaw_detune_fundamental_frequency_distribution()`

### 2. Pattern Frequency Parameters (✅ FIXED)
**Was**: No working tests
**Now**: ✅ Comprehensive FFT-based diagnostic test suite
**Tests**: `test_pattern_frequency_debug.rs` (8 tests, all passing)

### 3. Polyphony (✅ VERIFIED)
**Status**: ✅ Comprehensive test suite exists
**Tests**: `test_polyphony_64_voices.rs` (11 tests, all passing)
**Coverage**: 64 voices, voice stealing, overlap, reset, RMS verification

### 4. Multi-Output
**Current**: No integration tests
**Needed**: Verify multiple outputs work independently

---

## Recommended Next Steps

1. **Fix detune test** - Implement proper FFT peak detection
2. **Debug pattern frequency parameters** - Find why they don't work
3. **Implement 3-5 pattern transformations** - Start with `fast`, `slow`, `rev`
4. **Document what works** - Update README and docs to match reality

---

## Questions for User

1. **Pattern transformations**: Do you want `$` operator or something else?
2. **Tidal compatibility**: How important is it to match Tidal syntax exactly?
3. **Priority**: Focus on fixing existing features or adding new ones?
4. **Polyphony**: Are 64 voices enough, or do you need more?

---

## Conclusion

Phonon has a **solid, unique architecture** with features Tidal/Glicol can't match:
- Patterns as continuous control signals
- Sample-accurate timing
- Unified signal graph

But it's missing some **high-level features**:
- Pattern transformations
- Some parameter modulation doesn't work
- Needs better testing

**Recommendation**: Fix existing features first (pattern freq params, detune test), then add 3-5 essential transformations. Don't chase complete Tidal compatibility - embrace Phonon's unique strengths.
