# Phonon Development Roadmap

**Last Updated**: 2025-11-13
**Current Status**: ~85% feature-complete for core vision

---

## ✅ What's Working NOW

### Just Completed (This Session - 2025-11-13)
1. ✅ **Systematic Pattern Testing**: 58 comprehensive tests for pattern parameters
   - Dynamics: compressor, bitcrush (17 tests)
   - Modulation effects: chorus, flanger, phaser, tremolo, vibrato (24 tests)
   - Oscillators: sine, saw, square with FM/LFO modulation (17 tests)
   - All tests verify audio production + pattern modulation differs from constant
2. ✅ **API Verification**: Fixed parameter counts for all effects
   - compressor(threshold, ratio, attack, release, makeup_gain) - 5 params
   - bitcrush(bits, sample_rate) - 2 params
   - flanger(depth, rate, feedback) - 3 params
   - phaser(rate, depth, feedback, stages) - 4 params
3. ✅ **Test Suite**: 400+ tests passing total

### Previously Completed
1. ✅ **Pattern transformations**: `$`, `<|` operators with `fast`, `slow`, `rev`, `every`
2. ✅ **Bidirectional signal flow**: `#` and `<<` for routing
3. ✅ **--cycles parameter**: Now correctly accounts for tempo
4. ✅ **Complex pattern rendering**: `"bd sn hh*4 cp"` triggers all events correctly

### Core Features Working
- ✅ Voice-based sample playback (64 voices, polyphonic)
- ✅ Sample routing through effects: `s("bd sn") # lpf(2000, 0.8)`
- ✅ Pattern-controlled synthesis: `sine("110 220 440")`
- ✅ Pattern-controlled filters: `saw(55) # lpf("500 2000", 0.8)`
- ✅ Live coding with auto-reload (`phonon live`)
- ✅ Mini-notation: Euclidean rhythms, alternation, subdivision, rests
- ✅ Signal buses: `~lfo = sine(0.5)`
- ✅ Signal math: `~a + ~b`, `~osc * 0.5`
- ✅ Multi-output: `out1 = sine(110)`, `out2 = sine(220)`
- ✅ Hush/Panic: Backend for silencing and voice killing
- ✅ 187 tests passing (5 new multi-output tests)

---

## ❌ What's MISSING for Full Vision

### HIGH PRIORITY (Core Functionality)

#### 1. Multi-output System ✅ COMPLETE
**Status**: ✅ COMPLETE - Fully working in render and live modes
**Priority**: COMPLETE

**What's working**:
- ✅ `out1`, `out2`, etc. work in render mode
- ✅ `out1`, `out2`, etc. work in live mode
- ✅ UnifiedSignalGraph supports multiple outputs
- ✅ `hush_all()` and `hush_channel()` methods implemented
- ✅ `panic()` method implemented (kills voices + silences)
- ✅ `hush`, `hush N`, and `panic` command parsing in live mode
- ✅ 8 comprehensive tests passing (5 multi-output + 3 live commands)

**Completed**:
- ✅ Multi-output backend infrastructure
- ✅ Render mode parser integration
- ✅ Live mode parser integration with actual method calls
- ✅ All tests passing

---

#### 2. Sample Bank Selection ✅ COMPLETE
**Status**: ✅ Inline form working (final form)
**Priority**: COMPLETE - no 2-arg form needed

**What's working**:
```phonon
s "bd:0 bd:1 bd:2"           # ✅ Inline sample numbers WORK
s "bd:0 bd:1" $ fast 2       # ✅ Works with transforms
```

**Implementation completed**:
- ✅ Updated mini-notation parser to handle `:` in sample names (mini_notation_v3.rs:140)
- ✅ Parse `s "bd:0"` into sample name + number (sample_loader.rs already had this)
- ✅ SampleBank supports numbered sample lookup (existing functionality)
- ✅ Added comprehensive tests (tests/test_sample_bank_selection.rs)
- ✅ End-to-end audio rendering verification

**Tests passing**:
- ✅ Test `s "bd:0 bd:1 bd:2"` picks different samples
- ✅ Test mini-notation preserves colon syntax
- ✅ Test fallback behavior for out-of-range indices
- ✅ End-to-end audio rendering with sample selection

**Design decision**: Only inline form `s "bd:0"` will be supported. No 2-arg form needed.

---

#### 3. Pattern DSP Parameters ✅ COMPLETE
**Status**: ✅ COMPLETE - Fully implemented and tested
**Priority**: COMPLETE

Per-voice/per-event control using Tidal-style # chaining:
```phonon
s "bd sn" # gain "0.8 1.0" # pan "0 1"                              # ✅ WORKS
s "hh*16" # gain 0.5 # pan -1.0 # speed 2.0                         # ✅ WORKS
s "bd" # speed "1 0.5 2" # cut 1                                    # ✅ WORKS
s "bd*4" # attack 0.01 # release 0.2                                # ✅ WORKS
s "bd*4" # gain 0.8 # pan -0.3 # speed 0.9 # cut 1 # attack 0.01 # release 0.2  # ✅ WORKS
```

**Parameters implemented**:
- ✅ `gain` - amplitude control (0.0-1.0+)
- ✅ `pan` - stereo positioning (-1.0 = left, 1.0 = right)
- ✅ `speed` - playback rate (1.0 = normal, 0.5 = half, 2.0 = double)
- ✅ `cut` - cut groups (voice stealing)
- ✅ `attack` - envelope attack time (seconds)
- ✅ `release` - envelope release time (seconds)

**Implementation completed**:
- ✅ Kwargs parsing in `s()` function (compositional_compiler.rs:630-640)
- ✅ Per-voice parameter storage in VoiceManager (voice_manager.rs:99-122)
- ✅ Gain scaling per voice
- ✅ Pan (stereo positioning with equal-power panning)
- ✅ Speed (sample rate adjustment with pitch shifting)
- ✅ Cut groups (voice stealing by group)
- ✅ Envelope parameters (attack/release)
- ✅ 19 comprehensive tests (tests/test_dsp_parameters.rs)

**Tests passing** (19/19):
- ✅ Gain constant value and patterns
- ✅ Gain=0 produces silence
- ✅ Pan left, center, right
- ✅ Pan patterns
- ✅ Speed normal, double, half
- ✅ Speed patterns
- ✅ Cut groups basic and patterns
- ✅ Attack and release envelopes
- ✅ Multiple parameters combined
- ✅ All parameters with pattern control
- ✅ Parameters with transforms

---

### MEDIUM PRIORITY (Enhanced Functionality)

#### 4. More Effects ✅ COMPLETE
**Status**: ✅ COMPLETE - All basic effects implemented
**Priority**: COMPLETE

Implemented:
```phonon
~drums # reverb 0.5 0.8 0.3   # Reverb (room_size, damping, mix)
~bass # delay 0.25 0.6        # Delay (time, feedback)
~lead # distortion 2.0 0.5    # Distortion (drive, mix)
~mix # compressor 4.0 0.7     # Compressor (ratio, threshold)
~drums # bitcrush 8           # Bitcrusher (bits)
```

**Completed tasks**:
- ✅ Reverb (fundsp-based implementation)
- ✅ Delay (circular buffer)
- ✅ Distortion (waveshaping)
- ✅ Compressor (dynamics)
- ✅ Bitcrusher (sample rate reduction)
- ✅ Tests passing (test_reverb_stereo_integration, test_fundsp_reverb)

---

#### 5. MIDI Output
**Status**: Basic handler exists but not integrated
**Priority**: MEDIUM - useful for hardware integration

Need:
```phonon
midi("c4 e4 g4")               # Send pattern as MIDI notes
midi("c4 e4", velocity="64 127")
midi("c4 e4", channel=1)
```

**Implementation tasks**:
- [ ] Add `midi()` function to parser
- [ ] Wire to existing MidiOutputHandler
- [ ] Map pattern values to MIDI notes
- [ ] Support velocity patterns
- [ ] Support channel selection
- [ ] Add tests

**Estimated effort**: 1-2 days

---

#### 6. More Pattern Transformations ✅ COMPLETE
**Status**: ✅ COMPLETE - All advanced transformations implemented
**Priority**: COMPLETE

Implemented:
```phonon
"bd sn" $ jux rev             # Stereo manipulation (pattern_ops.rs)
"bd sn" $ stutter 3           # Repeat events 3 times
"bd sn" $ chop 4              # Sample slicing (pattern_ops_extended.rs)
"bd sn" $ degradeBy 0.3       # Probabilistic removal (30% chance)
"bd sn" $ scramble 4          # Randomize order (pattern_ops_extended.rs)
```

**Completed tasks**:
- ✅ `jux` - stereo manipulation (pattern_ops.rs)
- ✅ `stutter` - repeat events (pattern_ops.rs)
- ✅ `chop` - slice samples (pattern_ops_extended.rs)
- ✅ `degradeBy` - random removal (pattern_ops.rs)
- ✅ `scramble` - shuffle events (pattern_ops_extended.rs)
- ✅ Tests passing (test_pattern_transformations.rs - 8 tests)

---

### LOWER PRIORITY (Nice-to-Have)

#### 7. Pattern Buses
**Status**: Broken for Sample patterns
**Priority**: LOW - workaround exists (use signal buses)

Currently broken:
```phonon
~a: s("bd sn")
~b: ~a $ rev
out ~a + ~b * 0.5  # Produces silence
```

**Issue**: Pattern buses don't work the same way as signal buses

**Possible solutions**:
1. Make pattern buses evaluate and cache at sample rate
2. Remove pattern buses entirely (use signal buses only)
3. Document limitation and provide workarounds

**Estimated effort**: 1-2 days (if we fix it) or 1 hour (if we document it)

---

#### 8. REPL Improvements
**Status**: Experimental, basic functionality only
**Priority**: LOW - file-based workflow works fine

Improvements needed:
- Better error messages
- Tab completion for functions/buses
- Pattern preview (show what pattern will play)
- History navigation
- Multi-line editing

**Estimated effort**: 3-5 days

---

#### 9. Documentation Updates
**Status**: Outdated and incomplete
**Priority**: MEDIUM-HIGH - important for users

Tasks:
- [ ] Update PHONON_LANGUAGE_REFERENCE.md to use `=` instead of `:`
- [ ] Remove `$` references from old docs (now implemented)
- [ ] Add tutorial for beginners (QUICK_START.md)
- [ ] Add cookbook of common patterns
- [ ] Document what works vs. what's planned
- [ ] Add comparison to Tidal/Strudel (why Phonon is different)
- [ ] Update README with latest features

**Estimated effort**: 1-2 days

---

## 🎯 The Unique Vision

### What Makes Phonon Different

**Tidal/Strudel** (Event-based):
```haskell
d1 $ sound "bd sn"  # Triggers discrete events
```

**Phonon** (Signal-based):
```phonon
out = sine("110 220 440") * 0.2  # Pattern IS the control signal
```

### The Core Differentiator

In Phonon, **patterns evaluate at sample rate** (44.1kHz) and can modulate **any** synthesis parameter continuously. This is **impossible** in Tidal/Strudel where patterns only trigger discrete events.

### What You Can Do in Phonon (but not Tidal)

```phonon
tempo 1.0
~lfo = sine(0.25)                           # LFO pattern
~bass = saw("55 82.5 110")                  # Frequency pattern
out = ~bass # lpf(~lfo * 2000 + 500, 0.8) # Pattern modulates filter!
```

In Tidal, you can't use patterns to modulate synthesis parameters in real-time. Patterns only control when events are triggered. In Phonon, patterns ARE the control signals.

---

## 📋 Recommended Implementation Order

### Phase 1: Core Synthesis ✅ COMPLETE
1. ✅ **Multi-output system** - COMPLETE
2. ✅ **Sample selection** - COMPLETE (inline form only)
3. ✅ **Pattern DSP parameters** - COMPLETE
4. ✅ **Systematic testing** - COMPLETE (58 new tests)

### Phase 2: Synthesis Expansion (CURRENT PRIORITY)
**Focus**: More synthesis capabilities (NOT MIDI/OSC)

1. **FM Synthesis** (1-2 days) - HIGH PRIORITY
   - Implement FM oscillator with modulation index
   - Pattern-controlled FM synthesis
   - Tests for FM sounds

2. **Granular Synthesis** (2-3 days) - HIGH PRIORITY
   - Grain-based sample playback
   - Pattern-controlled grain parameters (size, density, pitch)
   - Tests for granular effects

3. **Wavetable Synthesis** (2-3 days) - HIGH PRIORITY
   - Wavetable oscillator with position scanning
   - Pattern-controlled wavetable position
   - Tests for wavetable sounds

4. **More Filters** (1-2 days) - MEDIUM PRIORITY
   - Moog ladder filter
   - State variable filter
   - Comb filter
   - All pattern-controllable

5. **More Effects** (1-2 days) - MEDIUM PRIORITY
   - Convolution reverb
   - Pitch shifter
   - Vocoder
   - Ring modulator

### Phase 3: Documentation & Polish (1-2 days)
6. **Update documentation** (1-2 days)
   - Fix outdated syntax references (`:` → `$` for outputs)
   - Remove 2-arg sample bank references
   - Add synthesis cookbook
   - Update language reference

### Phase 4: LOWER PRIORITY (Future)
7. **MIDI output** - DEPRIORITIZED
   - Hardware integration (when needed)

8. **OSC integration** - DEPRIORITIZED
   - Network communication (when needed)

9. **C integration** - DEPRIORITIZED
   - Embedding (when needed)

---

## Test-Driven Development Workflow

For each feature:

1. **Write failing test** that demonstrates desired behavior
2. **Run test** to confirm it fails
3. **Implement minimal code** to make test pass
4. **Run test** to confirm it passes
5. **Refactor** if needed
6. **Document** the feature

Example workflow:
```bash
# 1. Write test
tests/test_multi_output.rs

# 2. Run test (should fail)
cargo test test_multi_output

# 3. Implement feature
src/main.rs
src/unified_graph.rs

# 4. Run test (should pass)
cargo test test_multi_output

# 5. Commit
git add tests/test_multi_output.rs src/main.rs src/unified_graph.rs
git commit -m "Implement multi-output system with tests"
```

---

## Current Progress: ~90% Complete

**Working**:
- ✅ Pattern system (mini-notation)
- ✅ Voice-based sample playback
- ✅ Synthesis (oscillators, basic filters)
- ✅ Pattern-controlled synthesis
- ✅ Sample routing through effects
- ✅ Live coding workflow
- ✅ Pattern transformations (fast, slow, rev, every, jux, stutter, chop, degradeBy, scramble)
- ✅ Bidirectional operators (|>, <|, >>, <<)
- ✅ Multi-output system (render and live modes)
- ✅ Hush/Panic commands (render and live modes)
- ✅ Sample bank selection inline form: `s("bd:0 bd:1 bd:2")`
- ✅ Pattern DSP parameters (gain, pan, speed, cut, attack, release)
- ✅ Effects (reverb, delay, distortion, compressor, bitcrush)

**Current Priorities (Phase 2)**:
- ❌ FM Synthesis (HIGH - 1-2 days)
- ❌ Granular Synthesis (HIGH - 2-3 days)
- ❌ Wavetable Synthesis (HIGH - 2-3 days)
- ❌ More Filters: Moog, SVF, Comb (MEDIUM - 1-2 days)
- ❌ More Effects: Convolution reverb, pitch shifter, vocoder, ring mod (MEDIUM - 1-2 days)
- ❌ Updated docs (HIGH - 1-2 days)

**Deprioritized** (will implement later when needed):
- ⏸ MIDI output
- ⏸ OSC integration
- ⏸ C integration
- ⏸ Sample selection 2-arg form (not needed - inline form is final)

**Estimated time to Phase 2 complete**: 7-12 days for all synthesis expansion

---

## Design Decisions Made

1. ✅ **Outputs**: Multiple outputs working (out1, out2, etc.)
2. ✅ **Sample bank**: Inline form only (`s "bd:0"`) - no 2-arg form
3. ✅ **Pattern parameters**: ALL parameters accept patterns (P0.0 complete)
4. ✅ **Priority**: Synthesis expansion > MIDI/OSC/C integration
5. ✅ **Testing**: Systematic 3-level testing for all features
6. 🔄 **Output syntax**: Will change from `:` to `$` (e.g., `o1 $ s "bd"`) - NOT YET IMPLEMENTED

---

## Notes

- Architecture is solid - no major refactoring needed
- Focus on filling in missing conveniences
- Maintain test coverage as we add features
- Document what makes Phonon unique vs Tidal/Strudel
- Keep TDD workflow: test first, implement second
