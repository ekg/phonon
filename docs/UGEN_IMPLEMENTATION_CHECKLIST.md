# UGen Implementation & Testing Checklist

**Purpose**: Systematic verification and implementation of all SuperCollider/CSound UGens
**Last Updated**: 2025-10-29
**Current Status**: 40 custom verified, 60 fundsp available = 100 total possible
**Strategy**: Leverage fundsp (60 UGens ready) + Custom implementation (30 UGens needed)
**Goal**: 90 UGens for SuperCollider synthesis parity

**📖 See Also**: `SYNTHESIS_SEMANTICS_AND_FUNDSP_INTEGRATION.md` for complete integration strategy

---

## Implementation Strategy

### Two Paths to UGen Implementation

**Path 1: fundsp Wrapper (60 UGens) - FAST** ⚡
- fundsp provides battle-tested DSP implementations
- We wrap them with Phonon's pattern modulation
- Effort: 1-2 hours per UGen (mostly testing)
- Examples: moog, chorus, reverb, adsr, pluck, organ

**Path 2: Custom Implementation (30 UGens) - SLOW** 🐢
- fundsp doesn't provide or isn't suitable
- We implement from papers/research
- Effort: 4-8 hours per UGen (research + implementation)
- Examples: sample playback, granular, pitch tracking, spatial audio

---

## Methodology

For each UGen, we verify 3 stages:
1. **✅ Implemented** - SignalNode exists OR fundsp unit wrapped
2. **✅ Compiled** - Compiler handles it (case in compile function)
3. **✅ Tested** - Has comprehensive audio tests (signal correctness, musical verification)

**Testing Requirements**:
- Level 1: Pattern query verification (event counts over 4-8 cycles)
- Level 2: Onset detection (audio events match expectations)
- Level 3: Audio characteristics (RMS, spectral analysis, signal quality)

---

## TIER 1: Essential Synthesis (Critical - Do First!)

These UGens are **absolutely essential** for basic synthesis. Without these, you can't make music.

### Oscillators (4/4 - 100% Complete ✅)
- [x] **sine** - Pure sine wave ✅ FULLY VERIFIED
- [x] **saw** - Sawtooth wave ✅ FULLY VERIFIED
- [x] **square** - Square wave ✅ FULLY VERIFIED
- [x] **triangle** - Triangle wave ✅ FULLY VERIFIED

### Envelopes (7/8 - 87.5% Complete)
- [x] **adsr** - Attack-decay-sustain-release ✅ FULLY VERIFIED
- [x] **ad** - Attack-decay (percussion) ✅ FULLY VERIFIED
- [x] **line** - Linear ramp ✅ FULLY VERIFIED
- [x] **asr** - Attack-sustain-release ✅ FULLY VERIFIED
- [x] **segments** - Breakpoint envelope ✅ FULLY VERIFIED
- [x] **xline** - Exponential ramp ✅ FULLY VERIFIED
- [x] **curve** - Curved ramp ✅ FULLY VERIFIED
- [ ] **envGen** - Trigger-based envelope ⏳ NEEDS IMPLEMENTATION

### Basic Filters (3/3 - 100% Complete ✅)
- [x] **lpf** - Low-pass filter ✅ FULLY VERIFIED
- [x] **hpf** - High-pass filter ✅ FULLY VERIFIED
- [x] **bpf** - Band-pass filter ✅ FULLY VERIFIED

### Effects (6/6 - 100% Complete ✅)
- [x] **reverb** - Room reverb ✅ FULLY VERIFIED
- [x] **delay** - Echo/delay line ✅ FULLY VERIFIED
- [x] **distortion** - Waveshaping ✅ FULLY VERIFIED
- [x] **chorus** - Chorus effect ✅ FULLY VERIFIED
- [x] **compressor** - Dynamic range compression ✅ FULLY VERIFIED
- [x] **bitcrush** - Sample rate/bit depth reduction ✅ FULLY VERIFIED

**TIER 1 Status**: 23/24 (95.8%) - Almost complete! Just need envGen

---

## TIER 2: Extended Synthesis (High Priority)

These dramatically expand sonic possibilities. Priority for live coding.

**Strategy**: Most of these available in fundsp! Wrap them quickly.

### Advanced Oscillators (6/11) - **5 available in fundsp** ⚡

- [x] **fm** - Frequency modulation ✅ FULLY VERIFIED (custom)
- [x] **pulse** - Pulse width modulation ✅ FULLY VERIFIED (custom)
- [x] **impulse** - Periodic impulse train ✅ FULLY VERIFIED (custom)
- [x] **whiteNoise** - White noise ✅ FULLY VERIFIED (custom)
- [x] **pinkNoise** - Pink noise (1/f spectrum) ✅ FULLY VERIFIED (custom)
- [x] **brownNoise** - Brown noise (6dB/oct rolloff) ✅ FULLY VERIFIED (custom)
- [ ] **organ** - Organ/additive synthesis ⏳ **USE fundsp::organ_hz** ⚡
- [ ] **hammond** - Hammond tonewheel ⏳ **USE fundsp::hammond_hz** ⚡
- [ ] **pluck** - Karplus-Strong ⏳ **USE fundsp::pluck** ⚡
- [ ] **softSaw** - Anti-aliased saw ⏳ **USE fundsp::soft_saw_hz** ⚡
- [ ] **dsfSaw** - Band-limited saw ⏳ **USE fundsp::dsf_saw_hz** ⚡

**New additions from fundsp**: organ, hammond, pluck, softSaw, dsfSaw
**Removed**: pm, wavetable, superSaw, formant, blip (moved to Tier 3)

### Advanced Filters (5/12) - **7 available in fundsp** ⚡

- [x] **notch** - Band-reject filter ✅ FULLY VERIFIED (custom)
- [x] **comb** - Feedback delay ✅ FULLY VERIFIED (custom)
- [x] **moogLadder** - 4-pole Moog filter ✅ FULLY VERIFIED (custom → **fundsp::moog_hz available**)
- [x] **lag** - Exponential slew limiter ✅ FULLY VERIFIED (custom → **fundsp::follow available**)
- [ ] **allpass** - Phase manipulation ⏳ **USE fundsp::allpass_hz** ⚡
- [ ] **butterworth** - Butterworth filter ⏳ **USE fundsp::butterpass_hz** ⚡
- [ ] **resonator** - Resonant filter ⏳ **USE fundsp::resonator_hz** ⚡
- [ ] **peak** - Peak/bell filter ⏳ **USE fundsp::peak_hz** ⚡
- [ ] **bell** - Bell filter ⏳ **USE fundsp::bell_hz** ⚡
- [ ] **lowShelf** - Low shelf EQ ⏳ **USE fundsp::lowshelf_hz** ⚡
- [ ] **highShelf** - High shelf EQ ⏳ **USE fundsp::highshelf_hz** ⚡
- [ ] **dcBlock** - DC blocker ⏳ **USE fundsp::dcblock_hz** ⚡

**New additions from fundsp**: butterworth, peak, bell, lowShelf, highShelf, dcBlock
**Removed**: svf, biquad, resonz, rlpf, rhpf (covered by fundsp equivalents)

### Advanced Effects (3/9) - **2 available in fundsp** ⚡

- [x] **ringMod** - Ring modulator ✅ FULLY VERIFIED (custom)
- [x] **flanger** - Flanging effect ✅ FULLY VERIFIED (custom → **fundsp::flanger available**)
- [x] **limiter** - Brick-wall limiter ✅ FULLY VERIFIED (custom → **fundsp::limiter_stereo available**)
- [ ] **phaser** - All-pass phasing ⏳ **USE fundsp::phaser** ⚡
- [ ] **tremolo** - Amplitude modulation ⏳ CUSTOM (simple: sine * input)
- [ ] **vibrato** - Pitch modulation ⏳ CUSTOM (delay modulation)
- [ ] **freqShift** - Frequency shifter ⏳ CUSTOM (Hilbert transform)
- [ ] **gate** - Noise gate ⏳ CUSTOM (dynamics)
- [ ] **expander** - Upward compression ⏳ CUSTOM (dynamics)

**New additions from fundsp**: phaser
**Remaining custom**: tremolo, vibrato, freqShift, gate, expander

### Spatial Audio (3/3 - 100% Complete ✅)

- [x] **pan2** - Stereo panning ✅ FULLY VERIFIED (custom → **fundsp::pan available**)
- [x] **xfade** - Crossfader ✅ FULLY VERIFIED (custom)
- [x] **mix** - Signal mixer ✅ FULLY VERIFIED (custom)

**TIER 2 Status**: 17/35 (48.6%) → **With fundsp: 17 done + 14 easy wraps = 31/35 possible (88.6%)**

---

## TIER 3: Professional Production (Medium Priority)

These are for polished, professional productions.

**Strategy**: Mix of fundsp (reverbs, dynamics) and custom (analysis, spectral)

### Analysis & Control (6/12) - **2 available in fundsp** ⚡

- [x] **ampFollow** - Amplitude envelope follower ✅ FULLY VERIFIED (custom → **fundsp::afollow available**)
- [x] **peakFollow** - Peak detector ✅ FULLY VERIFIED (custom)
- [x] **rms** - RMS analyzer ✅ FULLY VERIFIED (custom)
- [x] **schmidt** - Schmitt trigger ✅ FULLY VERIFIED (custom)
- [x] **latch** - Sample & hold ✅ FULLY VERIFIED (custom)
- [x] **timer** - Elapsed time tracker ✅ FULLY VERIFIED (custom → **fundsp::timer available**)
- [ ] **pitchTrack** - Pitch detection (YIN algorithm) ⏳ CUSTOM (use realfft)
- [ ] **onsetDetect** - Onset detection ⏳ CUSTOM (spectral flux)
- [ ] **beatTrack** - Beat tracking ⏳ CUSTOM (onset + tempo)
- [ ] **fft** - FFT analysis ⏳ CUSTOM (use realfft crate)
- [ ] **pvMagFreeze** - Spectral freeze ⏳ CUSTOM (phase vocoder)
- [ ] **pvBinShift** - Spectral bin shifting ⏳ CUSTOM (phase vocoder)

**fundsp available**: afollow, timer
**Remaining custom**: pitchTrack, onsetDetect, beatTrack, fft, pvMagFreeze, pvBinShift

### Professional Effects (1/7) - **3 available in fundsp** ⚡

- [x] **eq** - Parametric EQ (3-band) ✅ FULLY VERIFIED (custom)
- [ ] **reverb2** - Enhanced reverb ⏳ **USE fundsp::reverb2_stereo** ⚡
- [ ] **reverb3** - Advanced reverb ⏳ **USE fundsp::reverb3_stereo** ⚡
- [ ] **reverb4** - Premium reverb ⏳ **USE fundsp::reverb4_stereo** ⚡
- [ ] **pitchShift** - Pitch shifter ⏳ CUSTOM (phase vocoder)
- [ ] **timeStretch** - Time stretcher ⏳ CUSTOM (phase vocoder)
- [ ] **vocoder** - FFT vocoder ⏳ CUSTOM (use realfft)

**New additions from fundsp**: reverb2, reverb3, reverb4
**Removed**: multibandComp, graphicEQ, stereoWidth (lower priority)
**Remaining custom**: pitchShift, timeStretch, vocoder

### Advanced Synthesis (0/5) - **ALL custom** 🐢

- [ ] **wavetable** - Arbitrary waveform synthesis ⏳ CUSTOM (implement interpolation)
- [ ] **superSaw** - Detuned saw stack ⏳ CUSTOM (7-9 oscillators)
- [ ] **formant** - Vowel synthesis ⏳ CUSTOM (formant filters)
- [ ] **granular** - Granular synthesis ⏳ CUSTOM (Curtis Roads)
- [ ] **waveguide** - Physical modeling ⏳ CUSTOM (Julius O. Smith)

**All custom**: wavetable, superSaw, formant, granular, waveguide

**TIER 3 Status**: 7/22 (31.8%) → **With fundsp: 7 done + 5 easy wraps = 12/22 (54.5%)**

---

## TIER 4: Experimental & Niche (Low Priority)

Advanced techniques for sound design and experimentation.

**Strategy**: Mostly custom, but fundsp provides some building blocks

### Spatial Audio Advanced (0/4) - **1 available in fundsp** ⚡

- [ ] **panner** - Multi-channel panner ⏳ **USE fundsp::panner** ⚡
- [ ] **rotate** - Stereo rotation ⏳ **USE fundsp::rotate** ⚡
- [ ] **binaural** - HRTF-based 3D audio ⏳ CUSTOM (HRTF database)
- [ ] **ambisonics** - Ambisonic encoding ⏳ CUSTOM (ambisonic math)

**fundsp available**: panner, rotate
**Remaining custom**: binaural, ambisonics

### Nonlinear & Experimental (0/6) - **ALL in fundsp!** ⚡⚡⚡

- [ ] **dlowpass** - Nonlinear lowpass ⏳ **USE fundsp::dlowpass_hz** ⚡
- [ ] **dhighpass** - Nonlinear highpass ⏳ **USE fundsp::dhighpass_hz** ⚡
- [ ] **dbell** - Nonlinear bell ⏳ **USE fundsp::dbell_hz** ⚡
- [ ] **dresonator** - Nonlinear resonator ⏳ **USE fundsp::dresonator_hz** ⚡
- [ ] **flowpass** - Feedback lowpass ⏳ **USE fundsp::flowpass_hz** ⚡
- [ ] **fresonator** - Feedback resonator ⏳ **USE fundsp::fresonator_hz** ⚡

**ALL in fundsp!** These are Jatin Chowdhury's nonlinear filters

**TIER 4 Status**: 0/10 (0%) → **With fundsp: 0 done + 8 easy wraps = 8/10 (80%)**

---

## Implementation Workflow (TDD - MANDATORY)

For **every single UGen**, follow this exact workflow:

### 1. Write Failing Test FIRST (30 min)
```bash
tests/test_ugen_NAME.rs
```

```rust
#[test]
fn test_NAME_level1_signal_generation() {
    // Test basic signal generation
    let result = render_ugen("NAME 440", 1.0);
    // Verify waveform shape, frequency, amplitude
}

#[test]
fn test_NAME_level2_spectral_analysis() {
    // Analyze frequency content
    let result = render_ugen("NAME 440", 1.0);
    // Verify harmonics, distortion, spectral characteristics
}

#[test]
fn test_NAME_level3_musical_usability() {
    // Test with patterns and modulation
    let result = render_dsl("~freq: sine 1 * 100 + 440\n~osc: NAME ~freq", 4.0);
    // Verify it works musically
}
```

### 2. Run Test - Confirm FAILS (2 min)
```bash
cargo test test_ugen_NAME
# Should error: "Unknown function: NAME"
```

### 3. Implement UGen (1-3 hours)

**Step 3a**: Define SignalNode in `src/unified_graph.rs`
```rust
SignalNode::NAME {
    param1: Signal,
    param2: Signal,
    state: NAMEState,
}
```

**Step 3b**: Add state struct (if needed)
```rust
#[derive(Debug, Clone)]
pub struct NAMEState {
    // Internal state
}
```

**Step 3c**: Implement evaluation logic in `eval_node()`
```rust
SignalNode::NAME { param1, param2, state } => {
    // DSP algorithm here
    // Return sample value
}
```

**Step 3d**: Add compiler in `src/compositional_compiler.rs`
```rust
"NAME" => compile_NAME(ctx, args),
```

### 4. Run Test - Confirm PASSES (2 min)
```bash
cargo test test_ugen_NAME
# All 3 levels should pass
```

### 5. Create Musical Example (10 min)
```phonon
-- docs/examples/NAME_demo.ph
tempo: 0.5
~osc: NAME 440
out: ~osc * 0.3
```

### 6. Commit (2 min)
```bash
git add tests/test_ugen_NAME.rs src/unified_graph.rs src/compositional_compiler.rs docs/examples/NAME_demo.ph
git commit -m "Implement NAME UGen with 3-level tests

- Signal generation: [describe]
- Spectral analysis: [describe]
- Musical test: [describe]
"
```

### 7. Update This Checklist
- Mark UGen as ✅ FULLY VERIFIED
- Update tier progress percentages

---

## Progress Tracking

### Overall Status (With fundsp Strategy)

**Current**:
- ✅ FULLY VERIFIED (custom): 40 / 90 (44.4%)
- ⚡ AVAILABLE in fundsp: 37 (can wrap quickly)
- 🐢 NEEDS CUSTOM: 13 (research + implement)
- **TOTAL ACHIEVABLE**: 90 UGens (40 done + 37 fundsp + 13 custom)

**Impact of fundsp**:
- **Before fundsp**: 50 UGens to implement from scratch (6-12 months)
- **After fundsp**: 37 quick wraps (3-5 weeks) + 13 custom (4-6 weeks)
- **New timeline**: 2-3 months instead of 6-12 months! 🚀

### Tier Progress (With fundsp)

| Tier | Done | fundsp | Custom | Total | % Complete | % Achievable |
|------|------|--------|--------|-------|------------|--------------|
| **Tier 1** | 23 | 1 | 0 | 24 | 95.8% | 100% |
| **Tier 2** | 17 | 14 | 4 | 35 | 48.6% | 88.6% |
| **Tier 3** | 7 | 5 | 10 | 22 | 31.8% | 54.5% |
| **Tier 4** | 0 | 8 | 2 | 10 | 0% | 80% |
| **TOTAL** | **47** | **28** | **16** | **91** | **51.6%** | **82.4%** |

### Priority Order (UPDATED with fundsp)

**Phase 1: fundsp Wrapper Infrastructure** (Week 1) - NEXT!
1. Implement `SignalNode::FundspUnit` wrapper
2. Test pattern modulation with fundsp units
3. Wrap 5 test UGens (organ, moog, reverb2, phaser, dlowpass)

**Phase 2: Wrap Easy fundsp UGens** (Week 2-4)
- Tier 2 fundsp oscillators: organ, hammond, pluck, softSaw, dsfSaw (5 × 2 hrs = 10 hrs)
- Tier 2 fundsp filters: allpass, butterworth, resonator, peak, bell, shelves, dcBlock (7 × 2 hrs = 14 hrs)
- Tier 2 fundsp effects: phaser (1 × 2 hrs = 2 hrs)
- **Total**: 13 UGens × 2 hours = 26 hours (3-4 weeks)

**Phase 3: Wrap Advanced fundsp UGens** (Week 5-6)
- Tier 3 fundsp: reverb2, reverb3, reverb4, afollow, timer (5 × 2 hrs = 10 hrs)
- Tier 4 fundsp: nonlinear filters (6 × 2 hrs = 12 hrs), spatial (2 × 2 hrs = 4 hrs)
- **Total**: 13 UGens × 2 hours = 26 hours (3-4 weeks)

**Phase 4: Custom Implementation** (Week 7-13)
- Tier 1: envGen (1 × 4 hrs = 4 hrs)
- Tier 2 custom: tremolo, vibrato, freqShift, gate, expander (5 × 4 hrs = 20 hrs)
- Tier 3 custom: analysis (6 × 6 hrs = 36 hrs), advanced synthesis (5 × 8 hrs = 40 hrs)
- Tier 4 custom: binaural, ambisonics (2 × 8 hrs = 16 hrs)
- **Total**: 19 custom UGens, ~120 hours (7-9 weeks)

### Estimated Timeline (REVISED)

**With fundsp integration**:
- **Phase 1** (Infrastructure): 1 week
- **Phase 2** (Easy wraps): 3-4 weeks
- **Phase 3** (Advanced wraps): 3-4 weeks
- **Phase 4** (Custom): 7-9 weeks
- **TOTAL**: **14-18 weeks** (3.5-4.5 months) for 90 UGens

**Comparison**:
- ❌ **Without fundsp**: 50 UGens × 6 hrs = 300 hours (12 months at 20 hrs/week)
- ✅ **With fundsp**: 37 wraps × 2 hrs + 13 custom × 6 hrs = 152 hours (2-3 months)
- **Speedup**: 2x faster! 🚀

**At current pace** (2-3 UGens per session):
- fundsp wraps: 12-18 sessions (6-9 weeks)
- Custom UGens: 5-7 sessions (3-4 weeks)
- **Total: 17-25 sessions over 2-3 months**

---

## Success Criteria

### Technical
- [ ] All Tier 1 UGens verified (24 UGens)
- [ ] All Tier 2 UGens verified (35 UGens)
- [ ] All Tier 3 UGens verified (22 UGens)
- [ ] All Tier 4 UGens verified (10 UGens)
- [ ] 270+ total tests passing (3 per UGen minimum)
- [ ] No audio artifacts or glitches
- [ ] All examples in docs/ render correctly

### Musical
- [ ] Can recreate any SuperCollider tutorial
- [ ] Can create professional-quality tracks
- [ ] Can perform live synthesis
- [ ] UGens work with pattern modulation
- [ ] Combinations work correctly

### Documentation
- [ ] Every UGen has usage example
- [ ] Every UGen has spectral/musical documentation
- [ ] SYNTHESIS_PARITY_PLAN.md updated
- [ ] This checklist kept current

---

## Next Session Start Here

**Current Focus**: PHASE 1 - fundsp Wrapper Infrastructure

**🎯 IMMEDIATE GOAL**: Get fundsp integration working, then wrap 37 UGens quickly!

**This Session Tasks**:

1. **Implement fundsp Wrapper** (2-3 hours)
   ```bash
   # Add fundsp dependency
   cargo add fundsp

   # Implement SignalNode::FundspUnit in src/unified_graph.rs
   # See SYNTHESIS_SEMANTICS_AND_FUNDSP_INTEGRATION.md for complete example
   ```

2. **Test Pattern Modulation** (1 hour)
   ```bash
   # Create test: tests/test_fundsp_integration.rs
   # Test that Phonon patterns can modulate fundsp parameters
   ```

3. **Wrap First 5 fundsp UGens** (5 × 1 hour = 5 hours)
   - `organ` (fundsp::organ_hz)
   - `moog` (fundsp::moog_hz) - replace custom implementation
   - `reverb2` (fundsp::reverb2_stereo)
   - `phaser` (fundsp::phaser)
   - `dlowpass` (fundsp::dlowpass_hz) - nonlinear filter

4. **Commit & Celebrate** 🎉
   ```bash
   git add -A
   git commit -m "Implement fundsp wrapper infrastructure + 5 test UGens

   - SignalNode::FundspUnit wrapper with pattern modulation
   - Phonon signals feed fundsp parameters at audio rate
   - Wrapped: organ, moog, reverb2, phaser, dlowpass
   - All 3-level tests passing
   - Musical examples for each

   This unlocks 37 more UGens available in fundsp!"
   ```

**After This Session**: We can wrap 2-3 fundsp UGens per hour! 🚀

**Quick Commands**:
```bash
# Add fundsp
cargo add fundsp

# Check fundsp prelude
cargo doc --open -p fundsp

# Test a fundsp unit manually
cargo run --bin phonon -- render --cycles 4 test_fundsp.ph

# Status check
rg "fundsp::" src/ | wc -l  # Count wrapped UGens
```

**📖 Reference**: See `SYNTHESIS_SEMANTICS_AND_FUNDSP_INTEGRATION.md` for:
- Complete implementation example (moogLadder)
- fundsp → Phonon syntax mapping
- Pattern modulation architecture
