# UGen Study-Implement-Test Workflow

**PURPOSE**: Enforce strict Study → Implement → Test cycle for EACH UGen individually
**LAST UPDATED**: 2025-10-29
**CRITICAL RULE**: Complete ALL three phases for ONE UGen before moving to the next

---

## ⚠️ CRITICAL WORKFLOW RULES

### DO THIS (Correct Workflow)

```
1. Pick ONE UGen from the list below
2. STUDY that UGen completely (research, understand algorithm)
3. IMPLEMENT that UGen with study context fresh in mind
4. TEST that UGen with implementation context fresh in mind
5. COMMIT that ONE UGen
6. Move to NEXT UGen
```

### DO NOT DO THIS (Wrong - Will Cause Confusion)

```
❌ Study 5 UGens → Implement all 5 → Test all 5
❌ Study multiple UGens at once
❌ Implement without completing study phase
❌ Test without fresh implementation context
❌ Batch anything together
❌ Simplify or skip phases
```

**WHY**: Study context must be fresh during implementation. Implementation context must be fresh during testing. Batching loses this context and causes bugs.

---

## Workflow Template (Copy for Each UGen)

### Phase 1: STUDY (30-60 minutes)

**Objective**: Understand the algorithm completely before touching code

**Tasks**:
1. **Read fundsp docs** (if using fundsp):
   - Function signature
   - Parameter meanings
   - Expected behavior

2. **Read SuperCollider source** (if custom):
   - Algorithm implementation
   - Edge cases handled
   - Parameter ranges

3. **Read academic papers** (if complex):
   - Original paper/algorithm
   - Mathematical formulation
   - Reference implementations

4. **Document understanding**:
   ```markdown
   ## Study Notes for [UGEN_NAME]

   **Algorithm**: [Describe in plain English]
   **Parameters**:
   - param1: [meaning, range, default]
   - param2: [meaning, range, default]

   **fundsp/SC mapping**: [How it's implemented elsewhere]
   **Edge cases**: [What to watch out for]
   **Expected behavior**: [What should happen]
   ```

5. **Create test expectations**:
   ```markdown
   ## Expected Test Behavior

   **Level 1 (Pattern query)**: [Expected event counts]
   **Level 2 (Onset detection)**: [Expected audio events]
   **Level 3 (Audio characteristics)**: [Expected RMS, spectral content]
   ```

**Completion Criteria**: ✅ Can explain algorithm without looking at docs

---

### Phase 2: IMPLEMENT (30-120 minutes)

**Objective**: Implement with study context fresh in mind

**Tasks**:
1. **Define SignalNode** (or fundsp wrapper):
   ```rust
   // In unified_graph.rs
   SignalNode::UGenName {
       param1: Signal,
       param2: Signal,
       state: UGenState,
   }
   ```

2. **Implement state** (if needed):
   ```rust
   #[derive(Debug, Clone)]
   pub struct UGenState {
       // From study phase: what state is needed?
   }
   ```

3. **Implement evaluation**:
   ```rust
   SignalNode::UGenName { param1, param2, state } => {
       // REFERENCE STUDY NOTES while implementing
       // Handle edge cases identified in study
   }
   ```

4. **Add compiler function**:
   ```rust
   "ugenname" => compile_ugenname(ctx, args),

   fn compile_ugenname(ctx: &mut CompilerContext, args: Vec<Expr>)
       -> Result<NodeId, String>
   {
       // Parse parameters per study notes
   }
   ```

5. **Document implementation**:
   ```rust
   /// [UGenName] - [One-line description]
   ///
   /// Algorithm: [From study notes]
   ///
   /// # Parameters
   /// - param1: [meaning from study]
   /// - param2: [meaning from study]
   ///
   /// # References
   /// - fundsp::[function] (if wrapper)
   /// - SC source: [file.cpp] (if ported)
   /// - Paper: [citation] (if from paper)
   ```

**Completion Criteria**: ✅ Code compiles, matches study understanding

---

### Phase 3: TEST (30-60 minutes)

**Objective**: Verify correctness with implementation context fresh

**Tasks**:
1. **Create test file**:
   ```bash
   touch tests/test_ugen_[name].rs
   ```

2. **Write Level 1 test** (Pattern query):
   ```rust
   #[test]
   fn test_[name]_level1_pattern_query() {
       // USE EXPECTED BEHAVIOR from study phase
       // Pattern events over 4-8 cycles

       let pattern = parse_mini_notation("...");
       let events = count_events_over_cycles(pattern, 8);
       assert_eq!(events, EXPECTED);  // From study phase
   }
   ```

3. **Write Level 2 test** (Onset detection):
   ```rust
   #[test]
   fn test_[name]_level2_onset_detection() {
       // USE EXPECTED BEHAVIOR from study phase

       let audio = render_dsl(code, duration);
       let onsets = detect_audio_events(&audio);
       assert_eq!(onsets.len(), EXPECTED);  // From study phase
   }
   ```

4. **Write Level 3 test** (Audio characteristics):
   ```rust
   #[test]
   fn test_[name]_level3_audio_quality() {
       // USE EXPECTED BEHAVIOR from study phase

       let audio = render_dsl(code, duration);
       let rms = calculate_rms(&audio);
       assert!(rms > THRESHOLD);  // From study phase

       // Spectral analysis if applicable
   }
   ```

5. **Write Level 4 test** (Comparative - IF fundsp equivalent exists):
   ```rust
   #[test]
   fn test_[name]_level4_fundsp_comparison() {
       // ONLY FOR UGENS WHERE FUNDSP EQUIVALENT EXISTS
       // Compare our custom implementation to fundsp's

       // Our implementation
       let code_ours = "out: saw 220 # [name] 1000 0.8";
       let audio_ours = render_dsl(code_ours, 1.0);

       // fundsp equivalent (via test wrapper)
       let audio_fundsp = render_fundsp_unit(
           fundsp::prelude::[fundsp_name](1000.0, 0.8),
           saw_input(220.0),
           44100
       );

       // Compare outputs - should be very similar
       let difference = calculate_difference(&audio_ours, &audio_fundsp);
       assert!(difference < 0.01,
           "Our implementation differs from fundsp by {:.3}%",
           difference * 100.0);

       println!("✅ [name] matches fundsp::[fundsp_name] within {:.3}%",
                difference * 100.0);
   }
   ```

   **When to write Level 4 test**:
   - ✅ Write for: moogLadder, lag, flanger, limiter, pan2, ampFollow, timer
   - ❌ Skip for: UGens without fundsp equivalent
   - **Purpose**: Verify our custom implementation is correct by comparing to battle-tested fundsp

6. **Run tests**:
   ```bash
   cargo test test_[name]
   # All 3 levels must pass
   ```

6. **Create musical example**:
   ```phonon
   -- docs/examples/[name]_demo.ph
   tempo: 2.0

   -- Demonstrate the UGen musically
   ~example: [use the UGen]
   out: ~example
   ```

7. **Verify example**:
   ```bash
   cargo run --bin phonon -- render --cycles 4 docs/examples/[name]_demo.ph /tmp/test.wav
   cargo run --bin wav_analyze -- /tmp/test.wav
   # Should match expected behavior from study phase
   ```

**Completion Criteria**: ✅ All 3 tests pass, musical example sounds correct

---

### Phase 4: COMMIT (5 minutes)

**Tasks**:
1. **Stage files**:
   ```bash
   git add tests/test_ugen_[name].rs
   git add src/unified_graph.rs
   git add src/compositional_compiler.rs
   git add docs/examples/[name]_demo.ph
   ```

2. **Commit with context**:
   ```bash
   git commit -m "Implement [UGenName] with 3-level verification

   STUDY:
   - Algorithm: [summary from study phase]
   - fundsp/SC mapping: [how it maps]
   - Expected behavior: [what should happen]

   IMPLEMENT:
   - SignalNode::[Name] variant
   - [Key implementation details]
   - [Edge cases handled]

   TEST:
   - Level 1: Pattern query ([result])
   - Level 2: Onset detection ([result])
   - Level 3: Audio quality ([result])
   - Musical example: [name]_demo.ph

   Source: fundsp::[function] / SC:[file] / Paper:[citation]"
   ```

3. **Update checklist**:
   - Mark UGen as ✅ FULLY VERIFIED in UGEN_IMPLEMENTATION_CHECKLIST.md

---

## UGen Implementation List (91 Total)

**Instructions**: Work through this list TOP TO BOTTOM. Complete Study → Implement → Test for ONE UGen before moving to next.

**Status Legend**:
- ⬜ NOT STARTED
- 📚 STUDYING
- 💻 IMPLEMENTING
- 🧪 TESTING
- ✅ COMPLETE

---

## TIER 1: Essential Synthesis (24 UGens)

### Oscillators (4 UGens)

#### 1. sine
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 2. saw
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 3. square
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 4. triangle
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

---

### Envelopes (8 UGens)

#### 5. adsr
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 6. ad
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 7. line
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 8. asr
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 9. segments
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 10. xline
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 11. curve
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 12. envGen - NEXT TO IMPLEMENT ⏳
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (trigger-based envelope)
- **fundsp equivalent**: None (custom logic needed)

**STUDY PHASE** (when starting):
```markdown
## Study Notes

**Algorithm**: Trigger-based envelope generator
- Monitors input signal for trigger (0→1 transition)
- On trigger, runs through envelope stages
- Stages: [levels] over [times] with [curves]

**SuperCollider mapping**: EnvGen.ar
- File: server/plugins/EnvGen.cpp
- Parameters: envelope, gate, levelScale, levelBias, timeScale

**Expected behavior**:
- Level 1: Pattern triggers generate envelope events
- Level 2: Audio shows envelope shapes at trigger times
- Level 3: Envelope matches specified ADSR/custom curve

**Edge cases**:
- Re-triggering before envelope completes
- Negative times (should error)
- Sustain stage handling
```

**IMPLEMENT PHASE** (do after study):
```rust
// Add to unified_graph.rs
SignalNode::EnvGen {
    trigger: Signal,
    levels: Vec<f32>,
    times: Vec<f32>,
    curves: Vec<f32>,
    state: EnvGenState,
}

#[derive(Debug, Clone)]
pub struct EnvGenState {
    current_stage: usize,
    stage_progress: f32,
    last_trigger: f32,
    current_level: f32,
}

// Evaluation logic
SignalNode::EnvGen { trigger, levels, times, curves, state } => {
    let trigger_val = self.eval_signal(*trigger, ...);

    // Detect trigger (0 → 1 transition)
    if trigger_val > 0.5 && state.last_trigger <= 0.5 {
        // Restart envelope
        state.current_stage = 0;
        state.stage_progress = 0.0;
    }
    state.last_trigger = trigger_val;

    // Advance envelope
    if state.current_stage < times.len() {
        let stage_dur = times[state.current_stage];
        state.stage_progress += 1.0 / (stage_dur * sample_rate);

        if state.stage_progress >= 1.0 {
            state.current_stage += 1;
            state.stage_progress = 0.0;
        }
    }

    // Interpolate level
    if state.current_stage < levels.len() - 1 {
        let start = levels[state.current_stage];
        let end = levels[state.current_stage + 1];
        let curve = curves[state.current_stage];

        // Curve interpolation (exponential if curve != 0)
        state.current_level = interpolate(start, end, state.stage_progress, curve);
    }

    state.current_level
}
```

**TEST PHASE** (do after implement):
```rust
// tests/test_ugen_envgen.rs

#[test]
fn test_envgen_level1_pattern_query() {
    let pattern = parse_mini_notation("x ~ x ~");  // Triggers on beat
    // Test that pattern generates correct events
}

#[test]
fn test_envgen_level2_onset_detection() {
    let code = r#"
        ~trig: s "bd ~ sn ~"
        ~env: ~trig # envGen [0.0 1.0 0.5 0.0] [0.01 0.1 0.2]
        out: sine 440 * ~env
    "#;
    let audio = render_dsl(code, 2.0);
    let onsets = detect_audio_events(&audio);
    assert_eq!(onsets.len(), 4);  // 2 triggers × 2 cycles
}

#[test]
fn test_envgen_level3_envelope_shape() {
    // Verify envelope follows specified curve
    let code = "~env: trigger # envGen [0 1 0] [0.1 0.1]";
    let audio = render_dsl(code, 0.5);

    // Check peak occurs at ~0.1s
    // Check returns to 0 at ~0.2s
}
```

**COMMIT** (after all tests pass):
```bash
git commit -m "Implement envGen trigger-based envelope

STUDY:
- Algorithm: Trigger detection → multi-stage envelope
- SC mapping: EnvGen.ar (server/plugins/EnvGen.cpp)
- Expected: Envelope follows levels/times on trigger

IMPLEMENT:
- SignalNode::EnvGen with trigger detection
- Multi-stage interpolation with curves
- Re-trigger handling

TEST:
- Level 1: Pattern triggers verified
- Level 2: 4 envelope events detected
- Level 3: Envelope shape matches specification
- Musical example: envgen_demo.ph"
```

---

### Basic Filters (3 UGens)

#### 13. lpf
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 14. hpf
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 15. bpf
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

---

### Effects (6 UGens)

#### 16. reverb
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 17. delay
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 18. distortion
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 19. chorus
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 20. compressor
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 21. bitcrush
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

---

### Sample Playback (3 UGens)

#### 22. sample (s)
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (voice manager)
- **Notes**: Already implemented and tested

#### 23. sampleBank
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (bank selection)
- **Notes**: Already implemented and tested

#### 24. sampleSpeed
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (playback rate)
- **Notes**: Already implemented and tested

---

## TIER 2: Extended Synthesis (35 UGens)

### Advanced Oscillators (11 UGens)

#### 25. fm
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 26. pulse
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 27. impulse
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 28. whiteNoise
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 29. pinkNoise
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 30. brownNoise
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)
- **Notes**: Already implemented and tested

#### 31. organ - IMPLEMENT AFTER envGen ⏳
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::organ_hz)

**STUDY PHASE**:
```markdown
## Study Notes

**Algorithm**: Additive synthesis with organ-like harmonics
**fundsp mapping**: `fundsp::prelude::organ_hz(frequency)`
**Parameters**:
- frequency: Base frequency in Hz

**Expected behavior**:
- Level 1: Pattern frequencies generate events
- Level 2: Onsets match pattern timing
- Level 3: Rich harmonic spectrum (fundamental + harmonics)

**fundsp docs**: https://docs.rs/fundsp/latest/fundsp/prelude/fn.organ_hz.html
```

**IMPLEMENT PHASE**: Follow fundsp wrapper template

**TEST PHASE**: 3-level tests + musical example

#### 32. hammond
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::hammond_hz)

#### 33. pluck
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::pluck)
- **Notes**: Karplus-Strong algorithm

#### 34. softSaw
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::soft_saw_hz)
- **Notes**: Anti-aliased sawtooth

#### 35. dsfSaw
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::dsf_saw_hz)
- **Notes**: Band-limited sawtooth via discrete summation

---

### Advanced Filters (12 UGens)

#### 36. notch
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)

#### 37. comb
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (keep existing)

#### 38. moogLadder
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (but fundsp::moog_hz available)
- **⚠️ TODO**: Add Level 4 comparative test vs fundsp::moog_hz

#### 39. lag
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (but fundsp::follow available)
- **⚠️ TODO**: Add Level 4 comparative test vs fundsp::follow

#### 40. allpass
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::allpass_hz)

#### 41. butterworth
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::butterpass_hz)

#### 42. resonator
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::resonator_hz)

#### 43. peak
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::peak_hz)

#### 44. bell
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::bell_hz)

#### 45. lowShelf
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::lowshelf_hz)

#### 46. highShelf
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::highshelf_hz)

#### 47. dcBlock
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::dcblock_hz)

---

### Advanced Effects (9 UGens)

#### 48. ringMod
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (no fundsp equivalent)

#### 49. flanger
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (but fundsp::flanger available)
- **⚠️ TODO**: Add Level 4 comparative test vs fundsp::flanger

#### 50. limiter
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (but fundsp::limiter_stereo available)
- **⚠️ TODO**: Add Level 4 comparative test vs fundsp::limiter_stereo

#### 51. phaser
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::phaser)

#### 52. tremolo
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (sine LFO × input)

#### 53. vibrato
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (delay modulation)

#### 54. freqShift
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (Hilbert transform)

#### 55. gate
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (dynamics - threshold-based)

#### 56. expander
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (dynamics - upward compression)

---

### Spatial Audio (3 UGens)

#### 57. pan2
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (but fundsp::pan available)
- **⚠️ TODO**: Add Level 4 comparative test vs fundsp::pan

#### 58. xfade
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (no fundsp equivalent)

#### 59. mix
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (no fundsp equivalent)

---

## TIER 3: Professional Production (22 UGens)

### Analysis & Control (12 UGens)

#### 60. ampFollow
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (but fundsp::afollow available)
- **⚠️ TODO**: Add Level 4 comparative test vs fundsp::afollow

#### 61. peakFollow
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (no fundsp equivalent)

#### 62. rms
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (no fundsp equivalent)

#### 63. schmidt
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (no fundsp equivalent)

#### 64. latch
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (no fundsp equivalent)

#### 65. timer
- **Status**: ✅ COMPLETE (custom implementation)
- **Type**: Custom (but fundsp::timer available)
- **⚠️ TODO**: Add Level 4 comparative test vs fundsp::timer

#### 66. pitchTrack
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (YIN algorithm + realfft)

#### 67. onsetDetect
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (spectral flux)

#### 68. beatTrack
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (onset detection + tempo estimation)

#### 69. fft
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (realfft crate)

#### 70. pvMagFreeze
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (phase vocoder)

#### 71. pvBinShift
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (phase vocoder)

---

### Professional Effects (7 UGens)

#### 72. eq
- **Status**: ✅ COMPLETE (custom implementation)

#### 73. reverb2
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::reverb2_stereo)

#### 74. reverb3
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::reverb3_stereo)

#### 75. reverb4
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::reverb4_stereo)

#### 76. pitchShift
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (phase vocoder)

#### 77. timeStretch
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (phase vocoder)

#### 78. vocoder
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (FFT-based)

---

### Advanced Synthesis (5 UGens)

#### 79. wavetable
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (table interpolation)

#### 80. superSaw
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (7-9 detuned saws)

#### 81. formant
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (formant filters)

#### 82. granular
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (Curtis Roads algorithm)

#### 83. waveguide
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (Julius O. Smith)

---

## TIER 4: Experimental & Niche (10 UGens)

### Spatial Audio Advanced (4 UGens)

#### 84. panner
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::panner)

#### 85. rotate
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::rotate)

#### 86. binaural
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (HRTF database)

#### 87. ambisonics
- **Status**: ⬜ NOT STARTED
- **Type**: Custom (ambisonic math)

---

### Nonlinear & Experimental (6 UGens)

#### 88. dlowpass
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::dlowpass_hz)
- **Notes**: Jatin Chowdhury's nonlinear lowpass

#### 89. dhighpass
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::dhighpass_hz)

#### 90. dbell
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::dbell_hz)

#### 91. dresonator
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::dresonator_hz)

#### 92. flowpass
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::flowpass_hz)

#### 93. fresonator
- **Status**: ⬜ NOT STARTED
- **Type**: fundsp wrapper (fundsp::fresonator_hz)

---

## Progress Tracking

**Total UGens**: 93
**Complete**: 47 (50.5%)
**In Progress**: 0 (0%)
**Not Started**: 46 (49.5%)

**Next UGen**: #12 envGen (Tier 1 - Essential)

---

## Session Workflow

### Starting a New Session

1. Open this file
2. Find the next ⬜ NOT STARTED UGen
3. Change status to 📚 STUDYING
4. Follow Study → Implement → Test phases
5. Change status to ✅ COMPLETE
6. Commit
7. Move to next UGen

### Example Session Flow

```
10:00 - Start session
10:01 - Find next UGen: #31 organ
10:02 - Change status to 📚 STUDYING
10:05 - Read fundsp docs for organ_hz
10:15 - Document study notes
10:20 - Change status to 💻 IMPLEMENTING
10:25 - Add SignalNode::Organ variant
10:40 - Add compiler function
10:50 - Change status to 🧪 TESTING
10:55 - Write 3-level tests
11:15 - Create musical example
11:20 - All tests pass!
11:25 - Commit with full context
11:30 - Change status to ✅ COMPLETE
11:31 - Move to next UGen: #32 hammond
```

---

## Critical Reminders

1. **ONE UGen at a time** - No batching!
2. **Study context fresh** - Implement immediately after studying
3. **Implementation context fresh** - Test immediately after implementing
4. **No simplification** - Full workflow every time
5. **Document everything** - Study notes, implementation details, test expectations
6. **Commit immediately** - Don't let multiple UGens pile up

**This workflow prevents confusion and ensures correctness.**
