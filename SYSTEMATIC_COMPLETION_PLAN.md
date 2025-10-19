# Systematic Completion Plan - Every Missing Piece

**Date**: 2025-10-18
**Goal**: Complete 100% of missing features with audio-verified tests
**Source**: HONEST_STATUS_REPORT_2025_10_18.md

---

## Completion Checklist

### Phase 1: Test Infrastructure (CRITICAL FOUNDATION)

#### 1.1 Fix Test Compilation Errors
- [ ] `tests/test_pattern_dsp_parameters.rs` - Missing API fields
- [ ] `tests/test_sample_integration.rs` - 11 compilation errors
- [ ] `tests/test_sample_pattern_operations.rs` - 7 compilation errors
- [ ] `tests/test_degrade_sample_node_comparison.rs` - 2 compilation errors
- [ ] `tests/test_scale_quantization.rs` - Compilation error
- [ ] `tests/test_tidal_style_syntax.rs` - Compilation error
- [ ] `tests/test_chained_transforms_dsl.rs` - Compilation error
- [ ] `tests/test_sample_speed_parameter.rs` - Compilation error
- [ ] `tests/test_pattern_transform_timing_verification.rs` - Compilation error
- [ ] `tests/test_tidal_patterns_comprehensive.rs` - Compilation error
- [ ] `tests/test_pattern_params_verification.rs` - Compilation error

**Success Criteria**: All test files compile

---

### Phase 2: Pattern DSP Parameters (HIGH PRIORITY - BLOCKING)

#### 2.1 Gain Parameter
- [ ] **Test**: `test_sample_gain_pattern.rs` - Pattern controls amplitude per event
- [ ] **Test**: `test_sample_gain_continuous.rs` - Continuous signal modulates gain
- [ ] **Implementation**: Parse `gain` keyword argument in `s()` function
- [ ] **Implementation**: Store gain pattern in Sample node
- [ ] **Implementation**: Query gain pattern at event time
- [ ] **Implementation**: Apply gain to voice in voice_manager
- [ ] **Audio verification**: Verify amplitude differences in rendered WAV

**DSL Examples**:
```phonon
out: s "bd sn" gain: "1.0 0.5"                    # Pattern gain
out: s "bd*4" gain: sine(0.25)                    # Continuous modulation
```

#### 2.2 Pan Parameter
- [ ] **Test**: `test_sample_pan_pattern.rs` - Pattern controls stereo position per event
- [ ] **Test**: `test_sample_pan_continuous.rs` - Continuous signal modulates pan
- [ ] **Implementation**: Parse `pan` keyword argument
- [ ] **Implementation**: Store pan pattern in Sample node
- [ ] **Implementation**: Query pan pattern at event time
- [ ] **Implementation**: Apply stereo panning in voice_manager
- [ ] **Audio verification**: Verify L/R channel differences in stereo WAV

**DSL Examples**:
```phonon
out: s "bd sn" pan: "-1 1"                        # Hard left, hard right
out: s "hh*8" pan: sine(0.5)                      # Auto-pan LFO
```

#### 2.3 Speed Parameter
- [ ] **Test**: `test_sample_speed_pattern.rs` - Pattern controls playback rate per event
- [ ] **Test**: `test_sample_speed_continuous.rs` - Continuous signal modulates speed
- [ ] **Implementation**: Parse `speed` keyword argument
- [ ] **Implementation**: Store speed pattern in Sample node
- [ ] **Implementation**: Query speed pattern at event time
- [ ] **Implementation**: Apply playback rate adjustment in voice_manager
- [ ] **Audio verification**: Verify frequency/duration changes in rendered WAV

**DSL Examples**:
```phonon
out: s "bd*4" speed: "1 0.5 2 1.5"                # Varying playback speeds
out: s "vocal" speed: sine(0.1) * 0.5 + 1.0       # Tape wobble effect
```

#### 2.4 Cut Parameter (Cut Groups)
- [ ] **Test**: `test_sample_cut_groups.rs` - Same cut group stops previous voices
- [ ] **Implementation**: Parse `cut` keyword argument
- [ ] **Implementation**: Store cut_group pattern in Sample node
- [ ] **Implementation**: Query cut_group pattern at event time
- [ ] **Implementation**: Kill voices with matching cut_group in voice_manager
- [ ] **Audio verification**: Verify voice stopping in rendered WAV

**DSL Examples**:
```phonon
out: s "hh*8 ho*2" cut: "1"                       # Closed hat stops open hat
```

#### 2.5 Attack Parameter (Envelope)
- [ ] **Test**: `test_sample_attack_pattern.rs` - Pattern controls attack time per event
- [ ] **Implementation**: Parse `attack` keyword argument
- [ ] **Implementation**: Store attack pattern in Sample node
- [ ] **Implementation**: Query attack pattern at event time
- [ ] **Implementation**: Apply attack envelope in voice_manager
- [ ] **Audio verification**: Verify envelope shape in rendered WAV

**DSL Examples**:
```phonon
out: s "bd*4" attack: "0.001 0.05 0.01 0.1"       # Varying attack times
```

#### 2.6 Release Parameter (Envelope)
- [ ] **Test**: `test_sample_release_pattern.rs` - Pattern controls release time per event
- [ ] **Implementation**: Parse `release` keyword argument
- [ ] **Implementation**: Store release pattern in Sample node
- [ ] **Implementation**: Query release pattern at event time
- [ ] **Implementation**: Apply release envelope in voice_manager
- [ ] **Audio verification**: Verify envelope shape in rendered WAV

**DSL Examples**:
```phonon
out: s "bd*4" release: "0.1 0.5 0.3 0.8"          # Varying release times
```

#### 2.7 N Parameter (Sample Selection)
- [ ] **Test**: `test_sample_n_pattern.rs` - Pattern selects sample numbers per event
- [ ] **Implementation**: Parse `n` keyword argument
- [ ] **Implementation**: Store n pattern in Sample node
- [ ] **Implementation**: Query n pattern at event time
- [ ] **Implementation**: Select sample based on n value
- [ ] **Audio verification**: Verify different samples triggered

**DSL Examples**:
```phonon
out: s "bd" n: "0 1 2 3"                          # Cycle through bd samples
```

#### 2.8 Note Parameter (Pitch)
- [ ] **Test**: `test_sample_note_pattern.rs` - Pattern controls pitch per event
- [ ] **Implementation**: Parse `note` keyword argument
- [ ] **Implementation**: Store note pattern in Sample node
- [ ] **Implementation**: Query note pattern at event time
- [ ] **Implementation**: Apply pitch shift via speed adjustment
- [ ] **Audio verification**: Verify pitch changes in rendered WAV

**DSL Examples**:
```phonon
out: s "vocal" note: "0 5 7 12"                   # Melodic sample playback
```

---

### Phase 3: Effects (HIGH PRIORITY - SOUND DESIGN)

#### 3.1 Reverb Effect
- [ ] **Test**: `test_reverb_effect.rs` - Reverb adds reflections to dry signal
- [ ] **Test**: `test_reverb_parameters.rs` - Room size and mix controls work
- [ ] **Implementation**: Add SignalNode::Reverb variant
- [ ] **Implementation**: Implement Freeverb algorithm
- [ ] **Implementation**: Parse `reverb(room_size, damping, mix)` in DSL
- [ ] **Implementation**: Wire reverb node in DslCompiler
- [ ] **Audio verification**: Verify increased tail length and spectral diffusion

**DSL Example**:
```phonon
out: s "bd sn" # reverb 0.5 0.5 0.3               # Room reverb
```

#### 3.2 Delay Effect
- [ ] **Test**: `test_delay_effect.rs` - Delay creates echoes at specified time
- [ ] **Test**: `test_delay_feedback.rs` - Feedback parameter controls repetitions
- [ ] **Implementation**: Add SignalNode::Delay variant
- [ ] **Implementation**: Implement circular buffer delay
- [ ] **Implementation**: Parse `delay(time, feedback, mix)` in DSL
- [ ] **Implementation**: Wire delay node in DslCompiler
- [ ] **Audio verification**: Verify echo timing and amplitude decay

**DSL Example**:
```phonon
out: s "sn" # delay 0.25 0.6 0.4                  # Quarter note delay
```

#### 3.3 Distortion Effect
- [ ] **Test**: `test_distortion_effect.rs` - Distortion adds harmonics
- [ ] **Test**: `test_distortion_amount.rs` - Drive parameter controls intensity
- [ ] **Implementation**: Add SignalNode::Distortion variant
- [ ] **Implementation**: Implement waveshaping/soft clipping
- [ ] **Implementation**: Parse `distort(drive, mix)` in DSL
- [ ] **Implementation**: Wire distortion node in DslCompiler
- [ ] **Audio verification**: Verify harmonic content increase

**DSL Example**:
```phonon
out: s "bd" # distort 0.7 0.5                     # Crunchy kick
```

#### 3.4 Bitcrush Effect
- [ ] **Test**: `test_bitcrush_effect.rs` - Bitcrush reduces bit depth
- [ ] **Test**: `test_bitcrush_sample_rate.rs` - Sample rate reduction works
- [ ] **Implementation**: Add SignalNode::Bitcrush variant
- [ ] **Implementation**: Implement bit depth reduction + sample rate reduction
- [ ] **Implementation**: Parse `crush(bits, rate_divisor)` in DSL
- [ ] **Implementation**: Wire bitcrush node in DslCompiler
- [ ] **Audio verification**: Verify aliasing and quantization noise

**DSL Example**:
```phonon
out: s "hh*16" # crush 8 4                        # Lo-fi hats
```

#### 3.5 Chorus Effect
- [ ] **Test**: `test_chorus_effect.rs` - Chorus creates detuned copies
- [ ] **Test**: `test_chorus_parameters.rs` - Rate and depth controls work
- [ ] **Implementation**: Add SignalNode::Chorus variant
- [ ] **Implementation**: Implement LFO-modulated delay lines
- [ ] **Implementation**: Parse `chorus(rate, depth, mix)` in DSL
- [ ] **Implementation**: Wire chorus node in DslCompiler
- [ ] **Audio verification**: Verify spectral widening and detuning

**DSL Example**:
```phonon
out: saw 55 # chorus 1.0 0.5 0.3                  # Wide bass
```

#### 3.6 Compressor Effect
- [ ] **Test**: `test_compressor_effect.rs` - Compressor reduces dynamic range
- [ ] **Test**: `test_compressor_parameters.rs` - Ratio and threshold work
- [ ] **Implementation**: Add SignalNode::Compressor variant
- [ ] **Implementation**: Implement RMS envelope follower + gain reduction
- [ ] **Implementation**: Parse `compress(ratio, threshold, attack, release)` in DSL
- [ ] **Implementation**: Wire compressor node in DslCompiler
- [ ] **Audio verification**: Verify reduced peak-to-RMS ratio

**DSL Example**:
```phonon
out: s "bd*4 sn*4" # compress 4.0 -12.0 0.01 0.1  # Punchy drums
```

---

### Phase 4: Pattern Transformations (MEDIUM PRIORITY)

#### 4.1 Jux Transform
- [ ] **Test**: `test_jux_transform.rs` - Jux applies transform to one stereo channel
- [ ] **Implementation**: Implement `jux` in pattern_ops.rs
- [ ] **Implementation**: Parse `$ jux(transform)` in DSL
- [ ] **Audio verification**: Verify L/R channel differences

**DSL Example**:
```phonon
out: s "bd sn" $ jux rev                          # Reversed on right channel
```

#### 4.2 Stut Transform (Stutter)
- [ ] **Test**: `test_stut_transform.rs` - Stut creates stuttering delays
- [ ] **Implementation**: Implement `stut` in pattern_ops.rs
- [ ] **Implementation**: Parse `$ stut(count, decay, time)` in DSL
- [ ] **Audio verification**: Verify echo repetitions

**DSL Example**:
```phonon
out: s "sn" $ stut 3 0.5 0.125                    # Snare stutter
```

#### 4.3 Chop Transform
- [ ] **Test**: `test_chop_transform.rs` - Chop slices samples into pieces
- [ ] **Implementation**: Implement `chop` in pattern_ops.rs
- [ ] **Implementation**: Parse `$ chop(n)` in DSL
- [ ] **Audio verification**: Verify sample slicing

**DSL Example**:
```phonon
out: s "break" $ chop 16                          # Slice breakbeat
```

#### 4.4 DegradeBy Transform
- [ ] **Test**: `test_degradeBy_transform.rs` - DegradeBy randomly removes events
- [ ] **Implementation**: Implement `degradeBy` in pattern_ops.rs
- [ ] **Implementation**: Parse `$ degradeBy(probability)` in DSL
- [ ] **Audio verification**: Verify event count reduction

**DSL Example**:
```phonon
out: s "hh*16" $ degradeBy 0.3                    # 30% chance to drop
```

#### 4.5 Scramble Transform
- [ ] **Test**: `test_scramble_transform.rs` - Scramble randomizes event order
- [ ] **Implementation**: Implement `scramble` in pattern_ops.rs
- [ ] **Implementation**: Parse `$ scramble` in DSL
- [ ] **Audio verification**: Verify event reordering

**DSL Example**:
```phonon
out: s "bd sn hh cp" $ scramble                   # Random order
```

---

### Phase 5: Sample Bank 2-Arg Form (MEDIUM PRIORITY)

#### 5.1 Two-Argument Sample Selection
- [ ] **Test**: `test_sample_2arg_form.rs` - `s("name", "pattern")` works
- [ ] **Implementation**: Parse 2-arg form of `s()` function
- [ ] **Implementation**: Apply pattern to sample number selection
- [ ] **Audio verification**: Verify different samples triggered

**DSL Example**:
```phonon
out: s "bd" "0 1 2 3"                             # Pattern for sample number
```

---

### Phase 6: Documentation Updates (MEDIUM-HIGH PRIORITY)

#### 6.1 README.md Updates
- [ ] Fix outdated test count (48 → 211+)
- [ ] Remove examples using non-existent effects (reverb/chorus)
- [ ] Update all DSL syntax examples to use correct colon syntax
- [ ] Add clear "What Works" vs "What's Coming" sections
- [ ] Update sample loading explanation (not streaming)

#### 6.2 QUICKSTART.md Creation
- [ ] Write "Hello World" example (simplest possible Phonon code)
- [ ] Write "First Beat" tutorial (basic drum pattern)
- [ ] Write "Adding Effects" tutorial (lpf/hpf)
- [ ] Write "Live Coding" tutorial (phonon live workflow)
- [ ] Write "Common Pitfalls" section (syntax errors, colon requirement)

#### 6.3 PHONON_LANGUAGE_REFERENCE.md Updates
- [ ] Update all operator examples to use current syntax
- [ ] Remove references to unimplemented features
- [ ] Add comprehensive effect reference (only lpf/hpf currently)
- [ ] Add pattern DSP parameter reference (once implemented)
- [ ] Add troubleshooting section

#### 6.4 Example Files Audit
- [ ] Verify all .ph files in examples/ use correct syntax
- [ ] Test all example files actually run without errors
- [ ] Add comments explaining what each example demonstrates
- [ ] Create examples/README.md with example descriptions

---

### Phase 7: MIDI Output (MEDIUM PRIORITY)

#### 7.1 MIDI Pattern Output
- [ ] **Test**: `test_midi_output.rs` - MIDI notes triggered by pattern
- [ ] **Implementation**: Parse `midi()` function in DSL
- [ ] **Implementation**: Wire to existing MidiOutputHandler
- [ ] **Implementation**: Map pattern values to MIDI notes
- [ ] **Verification**: Manual verification with MIDI monitor

**DSL Example**:
```phonon
midi "c4 e4 g4"                                   # Send MIDI notes
```

#### 7.2 MIDI Velocity Patterns
- [ ] **Test**: `test_midi_velocity.rs` - Velocity parameter works
- [ ] **Implementation**: Parse `velocity` keyword argument
- [ ] **Verification**: Manual verification with MIDI monitor

**DSL Example**:
```phonon
midi "c4 e4" velocity: "64 127"
```

#### 7.3 MIDI Channel Selection
- [ ] **Test**: `test_midi_channel.rs` - Channel parameter works
- [ ] **Implementation**: Parse `channel` keyword argument
- [ ] **Verification**: Manual verification with MIDI monitor

**DSL Example**:
```phonon
midi "c4" channel: 1
```

---

### Phase 8: REPL Improvements (LOWER PRIORITY)

#### 8.1 Better Error Messages
- [ ] Add line numbers to parser errors
- [ ] Add suggestions for common syntax mistakes
- [ ] Add "did you mean X?" for typos

#### 8.2 Tab Completion
- [ ] Complete function names (sine, saw, s, etc.)
- [ ] Complete bus names (~lfo, ~bass, etc.)
- [ ] Complete sample names (bd, sn, hh, etc.)

#### 8.3 Pattern Preview
- [ ] Show what events a pattern will trigger
- [ ] Show timing visualization

---

## Implementation Strategy

### For Each Feature:

1. **Write failing test FIRST**
   ```rust
   #[test]
   fn test_feature_name() {
       // Test demonstrates desired behavior
       // MUST FAIL initially
   }
   ```

2. **Run test to confirm failure**
   ```bash
   cargo test test_feature_name
   ```

3. **Implement minimal code**
   - Only write what's needed to pass test
   - Update src/unified_graph.rs, src/unified_graph_parser.rs, etc.

4. **Run test to confirm pass**
   ```bash
   cargo test test_feature_name
   ```

5. **Audio verification (for audio features)**
   ```rust
   use tests::audio_verification_enhanced::{
       verify_sample_playback,
       verify_dense_sample_pattern,
       analyze_wav_enhanced
   };
   ```

6. **Commit**
   ```bash
   git add tests/test_feature_name.rs src/*.rs
   git commit -m "Implement feature_name with audio-verified test"
   ```

---

## Progress Tracking

### Total Tasks: 95
- Phase 1 (Test Infrastructure): 11 tasks
- Phase 2 (Pattern DSP Parameters): 48 tasks (8 params × 6 tasks each)
- Phase 3 (Effects): 18 tasks (6 effects × 3 tasks each)
- Phase 4 (Pattern Transformations): 10 tasks (5 transforms × 2 tasks each)
- Phase 5 (Sample 2-Arg): 2 tasks
- Phase 6 (Documentation): 4 major sections
- Phase 7 (MIDI): 6 tasks
- Phase 8 (REPL): 3 tasks

### Estimated Completion Time
- **Phase 1**: 4-6 hours
- **Phase 2**: 2-3 days (CRITICAL PATH)
- **Phase 3**: 2-3 days (CRITICAL PATH)
- **Phase 4**: 2-3 days
- **Phase 5**: 4-6 hours
- **Phase 6**: 1-2 days
- **Phase 7**: 1-2 days
- **Phase 8**: 2-3 days

**TOTAL**: 2-3 weeks for 100% completion

---

## Success Criteria

### When Are We Done?

✅ **All test files compile**
✅ **All integration tests pass**
✅ **Every feature has audio-verified test**
✅ **README examples all work**
✅ **QUICKSTART.md tutorial is complete**
✅ **Every missing feature from report is implemented**

### Final Verification

```bash
# All tests pass
cargo test

# All examples run
for f in examples/*.ph; do
    phonon render "$f" "/tmp/$(basename $f .ph).wav" --duration 4 || echo "FAILED: $f"
done

# Documentation is accurate
# (Manual review of README, QUICKSTART, LANGUAGE_REFERENCE)
```

---

**Status**: Plan complete. Ready to execute systematically with TDD + audio verification.
