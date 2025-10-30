# Phonon Synthesis Semantics & fundsp Integration

**Purpose**: Define clear, functional semantics for signal composition in Phonon, leveraging fundsp while maintaining pattern-first philosophy
**Last Updated**: 2025-10-29
**Status**: Design Document - Ready for Implementation

---

## Executive Summary

**The Vision**: Phonon should have SuperCollider-level synthesis capabilities while keeping its unique pattern-first live-coding ergonomics.

**The Strategy**:
1. **Leverage fundsp** for 80% of DSP needs (60+ oscillators, filters, effects already battle-tested)
2. **Define clean Phonon syntax** that wraps fundsp's graph notation
3. **Maintain pattern modulation** as Phonon's killer feature
4. **Implement missing UGens** only when fundsp doesn't provide them

**Why This Works**:
- fundsp provides production-ready DSP implementations (MIT licensed)
- Phonon's architecture already supports dynamic signal graphs
- Pattern modulation of synthesis parameters is unique to Phonon
- We don't reinvent 30+ years of DSP research

---

## Core Principle: Patterns ARE Signals

**What makes Phonon unique**: Patterns can modulate synthesis parameters at audio rate.

```phonon
-- In Tidal/Strudel: Patterns only trigger discrete events
d1 $ s "bd sn" # cutoff 2000

-- In Phonon: Patterns ARE continuous control signals
~lfo: sine 0.25                      -- LFO pattern (0.25 Hz)
~cutoff: ~lfo * 2000 + 500           -- Modulate cutoff (500-2500 Hz)
out: saw 110 # lpf ~cutoff 0.8       -- Pattern controls filter!
```

This is **impossible** in other live coding systems. It's Phonon's superpower.

---

## Synthesis Semantics: Functional Signal Flow

### 1. Signal Composition Operators

Phonon uses **three operators** for signal composition:

#### **`#` - Signal Chain (Left to Right)**
```phonon
-- Chain signals through effects
out: sine 440 # lpf 1000 0.8 # reverb 0.3
--   ^^^^^^^^   ^^^^^^^^^^^^^   ^^^^^^^^^^
--   source     filter          effect
```

**Semantics**:
- Left-associative: `a # b # c` = `(a # b) # c`
- First argument becomes implicit input to next function
- Mirrors Unix pipe: `cat file | grep foo | wc`

#### **`$` - Pattern Transform**
```phonon
-- Apply pattern transformations
out: s "bd sn hh cp" $ fast 2 $ rev
--   ^^^^^^^^^^^^^^   ^^^^^^   ^^^
--   pattern          transform transform
```

**Semantics**:
- Right-associative: `a $ b $ c` = `a $ (b $ c)`
- Applies pattern operations (temporal, probabilistic, structural)
- Does NOT affect signal path, only event structure

#### **`*` / `+` / `-` - Signal Arithmetic**
```phonon
-- Combine signals mathematically
~lfo1: sine 0.1
~lfo2: sine 0.15
~combined: (~lfo1 + ~lfo2) * 0.5    -- Mix and scale
out: saw 220 * ~combined             -- Amplitude modulation
```

**Semantics**:
- Element-wise operation at sample rate
- Follow standard operator precedence
- Parentheses for grouping

### 2. Signal Types

```rust
// Internal representation
pub enum Signal {
    Constant(f32),              // Static value
    Node(NodeId),               // Computed signal (audio rate)
    Pattern(Box<Pattern<f32>>), // Pattern-derived signal
}
```

**All signals evaluate at audio rate (44.1 kHz)** - patterns are sampled continuously, not discretely.

---

## fundsp Integration Strategy

### Architecture: Wrapper Layer

```
┌─────────────────────────────────────┐
│  Phonon DSL (Space-separated)      │
│  "lpf 1000 0.8"                     │
└─────────────────┬───────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│  Phonon Compiler                    │
│  Parse → Type Check → Build Graph   │
└─────────────────┬───────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│  Unified Signal Graph               │
│  SignalNode enum variants           │
│  ┌──────────────────────────────┐  │
│  │ Custom Phonon DSP (30%)      │  │
│  │ - Pattern system integration │  │
│  │ - Sample playback            │  │
│  │ - Multi-output routing       │  │
│  └──────────────────────────────┘  │
│  ┌──────────────────────────────┐  │
│  │ fundsp Wrapper Nodes (70%)   │  │
│  │ - Oscillators, filters       │  │
│  │ - Effects, modulators        │  │
│  │ - Envelopes, dynamics        │  │
│  └──────────────────────────────┘  │
└─────────────────┬───────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│  Audio Output (44.1 kHz)            │
└─────────────────────────────────────┘
```

### Implementation Pattern: Wrapping fundsp Units

```rust
// In unified_graph.rs

// NEW: fundsp unit wrapper
SignalNode::FundspUnit {
    unit: Box<dyn fundsp::AudioUnit>,
    input: Signal,
    params: Vec<Signal>,      // Pattern-modulatable params!
    state: FundspState,
}

#[derive(Debug, Clone)]
pub struct FundspState {
    sample_rate: f32,
    inputs: Vec<f32>,         // Buffer for fundsp inputs
    outputs: Vec<f32>,        // Buffer for fundsp outputs
}

// Evaluation
SignalNode::FundspUnit { unit, input, params, state } => {
    // 1. Evaluate Phonon input signal
    let input_sample = self.eval_signal(input, ...);

    // 2. Evaluate Phonon parameter signals (pattern modulation!)
    for (i, param) in params.iter().enumerate() {
        state.inputs[i] = self.eval_signal(param, ...);
    }

    // 3. Feed to fundsp unit
    state.inputs[0] = input_sample;  // Audio input
    unit.tick(&state.inputs, &mut state.outputs);

    // 4. Return fundsp output
    state.outputs[0]
}
```

**Key Insight**: We wrap fundsp's `AudioUnit` trait, but parameters come from Phonon's pattern system. This gives us fundsp's quality with Phonon's expressiveness!

---

## Phonon DSL Syntax Design

### Principle: Space-Separated, Live-Coding Optimized

```phonon
-- ✅ CORRECT: Space-separated (Phonon style)
lpf 1000 0.8
reverb 0.3 0.5
moog 2000 0.7

-- ❌ WRONG: Parentheses/commas (not supported)
lpf(1000, 0.8)      -- ERROR
reverb(0.3, 0.5)    -- ERROR
```

**Why space-separated?**
- Fewer keystrokes = faster live coding
- Simpler parser = clearer errors
- Consistent with rest of Phonon DSL
- Mirrors shell commands

### Parameter Order: Audio-First

```phonon
-- General form: function [input] param1 param2 ...

-- Explicit input (useful for clarity)
lpf (saw 220) 1000 0.8

-- Implicit input via # chain (preferred)
saw 220 # lpf 1000 0.8
--        ^^^ input from left side

-- Pattern modulation
~cutoff: sine 0.1 * 1000 + 500
saw 220 # lpf ~cutoff 0.8
--            ^^^^^^^ pattern as parameter!
```

**Rule**: When using `#`, left side becomes implicit first argument.

---

## fundsp → Phonon Mapping

### Oscillators (fundsp has 20+, we expose the essential ones)

| fundsp Function | Phonon Syntax | Example |
|-----------------|---------------|---------|
| `sine_hz(440)` | `sine 440` | `sine 440` |
| `saw_hz(220)` | `saw 220` | `saw 220` |
| `square_hz(110)` | `square 110` | `square 110` |
| `triangle_hz(330)` | `triangle 330` | `triangle 330` |
| `pulse()` | `pulse 440 0.5` | `pulse 440 0.5` (freq, width) |
| `white()` | `whiteNoise` | `whiteNoise` |
| `pink()` | `pinkNoise` | `pinkNoise` |
| `brown()` | `brownNoise` | `brownNoise` |
| `organ_hz()` | `organ 440` | `organ 440` (additive) |
| `hammond_hz()` | `hammond 440` | `hammond 440` (tonewheel) |
| `pluck()` | `pluck 440 0.5` | `pluck 440 0.5` (Karplus-Strong) |
| `soft_saw_hz()` | `softSaw 440` | `softSaw 440` (anti-aliased) |
| `dsf_saw_hz()` | `dsfSaw 440` | `dsfSaw 440` (band-limited) |

**Implementation**:
```rust
// In compositional_compiler.rs
"sine" => compile_fundsp_osc(ctx, "sine_hz", args),
"saw" => compile_fundsp_osc(ctx, "saw_hz", args),
// etc.

fn compile_fundsp_osc(ctx: &mut CompilerContext, fundsp_fn: &str, args: Vec<Expr>)
    -> Result<NodeId, String>
{
    let freq = compile_signal(ctx, &args[0])?;
    let unit = match fundsp_fn {
        "sine_hz" => fundsp::prelude::sine_hz(440.0), // Placeholder freq
        "saw_hz" => fundsp::prelude::saw_hz(440.0),
        // etc.
    };
    ctx.add_fundsp_unit(unit, vec![freq])
}
```

### Filters (fundsp has 40+, we expose 15 most useful)

| fundsp Function | Phonon Syntax | Example |
|-----------------|---------------|---------|
| `lowpass_hz(fc, q)` | `lpf freq q` | `lpf 1000 0.8` |
| `highpass_hz(fc, q)` | `hpf freq q` | `hpf 500 0.7` |
| `bandpass_hz(fc, q)` | `bpf freq q` | `bpf 2000 2.0` |
| `notch_hz(fc, q)` | `notch freq q` | `notch 1000 5.0` |
| `allpass_hz(fc, q)` | `allpass freq q` | `allpass 440 1.0` |
| `peak_hz(fc, q, gain)` | `peak freq q gain` | `peak 1000 2.0 6.0` |
| `bell_hz(fc, q, gain)` | `bell freq q gain` | `bell 2000 1.0 3.0` |
| `lowshelf_hz(fc, q, gain)` | `lowShelf freq q gain` | `lowShelf 200 1.0 6.0` |
| `highshelf_hz(fc, q, gain)` | `highShelf freq q gain` | `highShelf 5000 1.0 -3.0` |
| `moog_hz(fc, q)` | `moogLadder freq q` | `moogLadder 2000 0.7` |
| `butterpass_hz(fc)` | `butterworth freq` | `butterworth 1000` |
| `resonator_hz(fc, bw)` | `resonator freq bw` | `resonator 440 50` |
| `dcblock_hz(fc)` | `dcBlock freq` | `dcBlock 10` |
| `pinkpass()` | `pinkFilter` | `white # pinkFilter` |

**Implementation** (with pattern modulation!):
```rust
"lpf" => compile_fundsp_filter(ctx, "lowpass_hz", args),

fn compile_fundsp_filter(ctx: &mut CompilerContext, fundsp_fn: &str, args: Vec<Expr>)
    -> Result<NodeId, String>
{
    let (input, params) = extract_chain_input(ctx, &args)?;
    let freq = compile_signal(ctx, &params[0])?;  // Can be pattern!
    let q = compile_signal(ctx, &params[1])?;     // Can be pattern!

    // Create fundsp unit with placeholder params
    let unit = match fundsp_fn {
        "lowpass_hz" => fundsp::prelude::lowpass_hz(1000.0, 1.0),
        // etc.
    };

    // Wrap with Phonon's pattern modulation
    ctx.add_fundsp_unit(unit, vec![input, freq, q])
}
```

**Magic**: Parameters are **Phonon signals**, so they can be patterns!

```phonon
~lfo: sine 0.2
~cutoff: ~lfo * 2000 + 500
out: saw 110 # lpf ~cutoff 0.8
--                  ^^^^^^^ Pattern modulation!
```

### Effects (fundsp has 10+, we expose all)

| fundsp Function | Phonon Syntax | Example |
|-----------------|---------------|---------|
| `reverb_stereo(r, t)` | `reverb room time` | `reverb 0.5 0.3` |
| `reverb2_stereo(...)` | `reverb2 ...` | (more params) |
| `chorus(s, d, fb)` | `chorus speed depth fb` | `chorus 0.5 0.02 0.5` |
| `flanger(s, d, fb)` | `flanger speed depth fb` | `flanger 0.2 0.005 0.7` |
| `phaser(s, d, fb)` | `phaser speed depth fb` | `phaser 0.3 0.5 0.6` |
| `delay(t)` | `delay time` | `delay 0.5` |
| `multitap(times)` | `multitap [times]` | `multitap "0.25 0.5 0.75"` |
| `limiter_stereo(t, a)` | `limiter time attack` | `limiter 0.01 0.001` |
| `oversample(u)` | `oversample N` | `oversample 2` |

### Envelopes (fundsp has ADSR, we extend it)

| fundsp Function | Phonon Syntax | Example |
|-----------------|---------------|---------|
| `adsr_live(a,d,s,r)` | `adsr a d s r` | `adsr 0.01 0.1 0.7 0.2` |
| `envelope(...)` | `envelope ...` | (breakpoint env) |
| `afollow(t)` | `ampFollow time` | `ampFollow 0.05` |
| `follow(t)` | `follow time` | `follow 0.1` |
| `declick()` | `declick` | `declick` |

**Phonon Extension** (not in fundsp, custom implementation):
```phonon
-- Trigger-based envelopes for sample playback
~trigger: s "bd ~ sn ~"
~env: ~trigger # adsr 0.001 0.1 0.0 0.2
out: sine 440 * ~env

-- Pattern-based envelopes (unique to Phonon!)
~env: envelope "0.0 1.0 0.5 0.0" 0.5
out: saw 220 * ~env
```

### Noise (fundsp has 5, we expose all)

| fundsp Function | Phonon Syntax | Notes |
|-----------------|---------------|-------|
| `white()` | `whiteNoise` | Full spectrum |
| `pink()` | `pinkNoise` | 1/f spectrum |
| `brown()` | `brownNoise` | 6dB/oct rolloff |
| `mls()` | `mls` | Maximum length sequence |
| `noise()` | `noise seed` | Seedable white noise |

---

## Custom Phonon Implementations (Not in fundsp)

### 1. Sample Playback (Core Phonon Feature)
```phonon
-- fundsp does NOT handle samples, we do this
s "bd sn hh cp"
sample "kick.wav"
sample "synth.wav" 0.5 1.0  -- start, speed
```

**Why Custom**:
- Needs voice manager (64 polyphonic voices)
- Needs sample bank system
- Needs Phonon pattern integration

### 2. Multi-Output Routing
```phonon
-- fundsp is single-output, Phonon needs multiple buses
out1: saw 110 # lpf 500 0.8
out2: s "bd sn" # reverb 0.5
out3: sine 220
```

**Why Custom**: Routing architecture unique to Phonon

### 3. Pattern-Specific UGens
```phonon
-- These make sense in pattern context, not in fundsp
euclid 3 8      -- Euclidean rhythm
degrade 0.5     -- Random removal
```

**Why Custom**: Pattern operations, not signal processing

### 4. Analysis UGens (fundsp has some, we extend)
```phonon
pitchTrack input      -- Pitch detection (YIN algorithm)
onsetDetect input     -- Onset detection
beatTrack input       -- Beat tracking
fft input size        -- FFT analysis
```

**Why Custom**: Real-time analysis needs, some not in fundsp

---

## SuperCollider UGen → Implementation Strategy

### Category 1: Use fundsp Directly (60 UGens)

**Oscillators**: sine, saw, square, triangle, pulse, organ, hammond, pluck, etc.
**Filters**: lpf, hpf, bpf, notch, allpass, moog, butterworth, resonator, etc.
**Effects**: reverb, chorus, flanger, phaser, delay, limiter
**Envelopes**: adsr, envelope, follow, declick
**Noise**: white, pink, brown, mls

**Effort**: Low (1-2 hours each) - Just wrap fundsp units

### Category 2: Custom Implementation Required (30 UGens)

**Sample Playback**: Already implemented (voice_manager.rs)
**Pattern Operations**: Already implemented (pattern_ops.rs)
**Analysis**: pitchTrack, onsetDetect, beatTrack, fft (use realfft crate)
**Spatial**: pan2, pan4, rotate, binaural (custom or use rust-spatial-audio)
**Physical Modeling**: karplusStrong (extend fundsp::pluck), waveguide (custom)
**Granular**: grain (custom, study Curtis Roads)

**Effort**: Medium-High (4-8 hours each) - Original DSP implementation

### Category 3: Not Needed / Low Priority (160 UGens)

**Trigger-based**: Many SC UGens assume discrete triggers, Phonon uses continuous patterns
**Server-specific**: Language control, OSC communication (not applicable)
**Duplicate variants**: SC has LFSaw, LFSaw.ar, LFSaw.kr - Phonon unifies these

**Effort**: None - Skip or defer

---

## Implementation Roadmap

### Phase 1: fundsp Wrapper Infrastructure (Week 1)

**Goal**: Make fundsp units usable from Phonon DSL

**Tasks**:
1. ✅ Research fundsp API (done)
2. Add `SignalNode::FundspUnit` variant
3. Implement fundsp evaluation in `eval_node()`
4. Add compiler functions for oscillators
5. Test pattern modulation of fundsp parameters

**Deliverable**:
```phonon
-- This works!
~lfo: sine 0.1
out: (fundsp::saw 220) # (fundsp::lowpass ~lfo*1000+500 0.8)
```

**Validation**: 3-level tests for 5 fundsp UGens

### Phase 2: Map Essential UGens to fundsp (Week 2-3)

**Goal**: 20 most-used UGens working via fundsp

**Priority List**:
1. Oscillators: sine, saw, square, triangle, pulse ✅ (already have custom, keep)
2. **NEW Oscillators**: organ, hammond, pluck, softSaw, dsfSaw
3. **Advanced Filters**: moog, butterworth, resonator, peak, bell, shelves
4. **Effects**: reverb, chorus, flanger, phaser (use fundsp implementations)
5. **Envelopes**: adsr (fundsp::adsr_live)

**Implementation Pattern**:
```bash
# For each UGen:
1. Write test (test_ugen_organ.rs)
2. Add compiler case ("organ" => compile_fundsp_osc)
3. Test pattern modulation
4. Create musical example (docs/examples/organ_demo.ph)
5. Commit
```

**Validation**: All Tier 1 UGens (24 total) working

### Phase 3: Complete fundsp Integration (Week 4-6)

**Goal**: All 60 fundsp-mapped UGens available

**Categories**:
- All oscillators (20)
- All filters (15)
- All effects (10)
- All envelopes (5)
- All noise (5)
- All dynamics (5)

**Validation**: 180+ tests passing (60 UGens × 3 levels)

### Phase 4: Custom UGen Implementation (Week 7-16)

**Goal**: Implement 30 UGens that fundsp doesn't provide

**Priority Order**:
1. Analysis: pitchTrack, onsetDetect, fft (use realfft)
2. Spatial: pan2, pan4, rotate (custom stereo/quad)
3. Granular: grain (study Curtis Roads, implement from papers)
4. Physical Modeling: waveguide (Julius O. Smith's work)
5. Advanced Effects: vocoder (FFT-based), pitchShift (phase vocoder)

**Validation**: 90+ tests (30 UGens × 3 levels)

### Phase 5: Polish & Documentation (Week 17-20)

**Goal**: Production-ready synthesis system

**Tasks**:
1. Comprehensive documentation for all 90 UGens
2. Musical examples demonstrating each UGen
3. Performance optimization (SIMD, multi-threading)
4. Regression testing (all 270+ tests passing)
5. Video tutorials

**Deliverable**: "Phonon achieves SuperCollider parity"

---

## Testing Strategy: Three-Level Verification (Mandatory)

**Every single UGen must pass all three levels**:

### Level 1: Pattern Query Verification
```rust
#[test]
fn test_moog_level1_pattern_query() {
    // Verify pattern events are correct
    let pattern = parse_mini_notation("110 220 440");
    let filtered = pattern.moog_ladder(2000.0, 0.7);

    let events = filtered.query(&state);
    assert_eq!(events.len(), 3);
}
```

### Level 2: Onset Detection (Audio Events)
```rust
#[test]
fn test_moog_level2_onset_detection() {
    let code = "out: sine \"110 220\" # moogLadder 2000 0.7";
    let audio = render_dsl(code, 2.0);

    let onsets = detect_audio_events(&audio);
    assert_eq!(onsets.len(), 4);  // 2 notes × 2 cycles
}
```

### Level 3: Audio Characteristics (Signal Quality)
```rust
#[test]
fn test_moog_level3_spectral_content() {
    let code = "out: saw 110 # moogLadder 500 0.8";
    let audio = render_dsl(code, 1.0);

    let spectrum = fft_analyze(&audio);
    // Moog filter has characteristic rolloff
    assert_peak_near(&spectrum, 110.0);      // Fundamental
    assert!(spectrum.energy_above(500.0) < 0.1);  // Cutoff works
}
```

**Why three levels?**
1. Pattern query catches logic bugs
2. Onset detection catches timing/silence bugs
3. Spectral analysis catches DSP bugs

---

## Example: Complete UGen Implementation

### Adding "moogLadder" (fundsp-based)

**Step 1: Write Test** (tests/test_ugen_moog.rs)
```rust
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use pattern_verification_utils::{render_dsl, detect_audio_events, fft_analyze};

#[test]
fn test_moog_level1_pattern_query() {
    let pattern = parse_mini_notation("110 220 440");
    let filtered = pattern.moog_ladder(2000.0, 0.7);

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let events = filtered.query(&state);
    assert_eq!(events.len(), 3, "Should have 3 notes");
}

#[test]
fn test_moog_level2_onset_detection() {
    let code = "~saw: saw \"110 220\"\nout: ~saw # moogLadder 2000 0.7";
    let audio = render_dsl(code, 2.0);  // 2 cycles

    let onsets = detect_audio_events(&audio, 44100.0, 0.01);
    assert_eq!(onsets.len(), 4, "Should have 4 onsets (2 notes × 2 cycles)");
}

#[test]
fn test_moog_level3_filtering() {
    let code = "out: saw 110 # moogLadder 500 0.8";
    let audio = render_dsl(code, 1.0);

    let spectrum = fft_analyze(&audio);

    // Should have fundamental
    assert_peak_near(&spectrum, 110.0, 10.0);

    // Should filter high frequencies
    let high_energy = spectrum.energy_in_range(1000.0, 5000.0);
    assert!(high_energy < 0.1, "High frequencies should be filtered");
}

#[test]
fn test_moog_pattern_modulation() {
    // Pattern modulates filter cutoff
    let code = r#"
        ~lfo: sine 2
        ~cutoff: ~lfo * 1000 + 1500
        out: saw 110 # moogLadder ~cutoff 0.7
    "#;
    let audio = render_dsl(code, 2.0);

    let rms = calculate_rms(&audio);
    assert!(rms > 0.05, "Should have audible output with modulation");
}
```

**Step 2: Add SignalNode Variant** (src/unified_graph.rs)
```rust
pub enum SignalNode {
    // ... existing variants ...

    MoogLadder {
        input: Signal,
        cutoff: Signal,      // Pattern-modulatable!
        resonance: Signal,   // Pattern-modulatable!
        state: MoogState,
    },
}

#[derive(Debug, Clone)]
pub struct MoogState {
    fundsp_unit: Box<dyn fundsp::AudioUnit>,
    sample_rate: f32,
    inputs: [f32; 3],      // [audio, cutoff, resonance]
    outputs: [f32; 1],
}
```

**Step 3: Implement Evaluation** (src/unified_graph.rs)
```rust
impl UnifiedSignalGraph {
    fn eval_node(&mut self, node_id: NodeId, ...) -> f32 {
        match &self.nodes[node_id.0] {
            // ... existing cases ...

            SignalNode::MoogLadder { input, cutoff, resonance, state } => {
                // 1. Evaluate Phonon signals
                let input_sample = self.eval_signal(*input, ...);
                let cutoff_val = self.eval_signal(*cutoff, ...);
                let resonance_val = self.eval_signal(*resonance, ...);

                // 2. Update fundsp unit parameters
                // (fundsp::moog_hz internally updates on parameter change)
                state.inputs[0] = input_sample;
                state.inputs[1] = cutoff_val;
                state.inputs[2] = resonance_val;

                // 3. Process through fundsp
                let mut unit = state.fundsp_unit.clone();
                unit.set_parameter(0, cutoff_val);
                unit.set_parameter(1, resonance_val);
                unit.tick(&[input_sample], &mut state.outputs);

                // 4. Return output
                state.outputs[0]
            }
        }
    }
}
```

**Step 4: Add Compiler** (src/compositional_compiler.rs)
```rust
fn compile_function(...) -> Result<NodeId, String> {
    match function_name.as_str() {
        // ... existing cases ...

        "moogLadder" => compile_moog_ladder(ctx, args),

        // ... rest ...
    }
}

fn compile_moog_ladder(ctx: &mut CompilerContext, args: Vec<Expr>)
    -> Result<NodeId, String>
{
    // Extract input from chain (if using #)
    let (input_signal, params) = extract_chain_input(ctx, &args)?;

    // Compile parameters (can be patterns!)
    let cutoff = if params.len() > 0 {
        compile_signal(ctx, &params[0])?
    } else {
        return Err("moogLadder requires cutoff parameter".to_string());
    };

    let resonance = if params.len() > 1 {
        compile_signal(ctx, &params[1])?
    } else {
        Signal::Constant(0.5)  // Default resonance
    };

    // Create fundsp unit
    let unit = fundsp::prelude::moog_hz(1000.0, 0.5);

    // Create state
    let state = MoogState {
        fundsp_unit: Box::new(unit),
        sample_rate: 44100.0,
        inputs: [0.0; 3],
        outputs: [0.0; 1],
    };

    // Add node
    let node = SignalNode::MoogLadder {
        input: input_signal,
        cutoff,
        resonance,
        state,
    };

    Ok(ctx.add_node(node))
}
```

**Step 5: Create Example** (docs/examples/moog_demo.ph)
```phonon
-- Moog Ladder Filter Demo
-- Classic analog-style low-pass filter with resonance

tempo: 2.0

-- Static filter
~bass1: saw 55 # moogLadder 500 0.8
out: ~bass1 * 0.3

-- Pattern-controlled cutoff
~lfo: sine 0.25
~cutoff: ~lfo * 2000 + 500
~bass2: saw 110 # moogLadder ~cutoff 0.7
out: ~bass2 * 0.3

-- Self-oscillation (high resonance)
~bass3: saw 82.5 # moogLadder 1000 0.95
out: ~bass3 * 0.2
```

**Step 6: Test**
```bash
cargo test test_moog
# All 4 tests should pass

cargo run --bin phonon -- render --cycles 4 docs/examples/moog_demo.ph /tmp/moog.wav
cargo run --bin wav_analyze -- /tmp/moog.wav
# Should show filtered output
```

**Step 7: Commit**
```bash
git add tests/test_ugen_moog.rs src/unified_graph.rs src/compositional_compiler.rs docs/examples/moog_demo.ph
git commit -m "Implement moogLadder filter via fundsp integration

- Wraps fundsp::moog_hz with Phonon pattern modulation
- Supports pattern-controlled cutoff and resonance
- Three-level tests: pattern query, onset detection, spectral analysis
- Musical example demonstrates static and modulated filtering
- Classic Moog ladder topology (4-pole 24dB/oct)

Closes #XX (if tracking issue)"
```

**Step 8: Update Checklist**
```bash
# In docs/UGEN_IMPLEMENTATION_CHECKLIST.md
- [x] **moogLadder** - 4-pole Moog filter ✅ FULLY VERIFIED
```

**Total time**: ~2 hours (mostly testing and documentation)

---

## Success Metrics

### Technical Metrics

**Coverage**:
- [ ] 60 fundsp-wrapped UGens working
- [ ] 30 custom UGens implemented
- [ ] 90 total UGens (SuperCollider parity)
- [ ] 270+ tests passing (90 UGens × 3 levels)

**Performance**:
- [ ] Real-time on consumer hardware
- [ ] fundsp units add <5% overhead vs custom implementations
- [ ] Pattern modulation works at audio rate (44.1 kHz)

### Musical Metrics

**Capabilities**:
- [ ] Can make professional techno/house tracks
- [ ] Analog-style synthesis (Moog, TB-303 emulation)
- [ ] FM synthesis (DX7-style bells, brass)
- [ ] Granular synthesis (ambient soundscapes)
- [ ] Physical modeling (plucked strings, waveguides)

### Integration Metrics

**Syntax**:
- [ ] Space-separated syntax consistent across all UGens
- [ ] `#` chaining works for all effects/filters
- [ ] Pattern modulation works for all parameters
- [ ] Error messages are clear and actionable

---

## FAQ

### Q: Why fundsp instead of implementing everything custom?

**A**:
1. **Quality**: fundsp has battle-tested DSP implementations
2. **Performance**: SIMD-accelerated, highly optimized
3. **Maintenance**: Community-maintained, bugs fixed upstream
4. **Time**: 60 UGens in weeks instead of months
5. **Licensing**: MIT licensed, compatible with Phonon

### Q: Will fundsp units be as expressive as custom Phonon DSP?

**A**: Yes! The key is our wrapper layer - fundsp provides the DSP algorithm, but **parameters come from Phonon's pattern system**. This gives us fundsp's quality with Phonon's expressiveness.

```phonon
-- This is fundsp::moog_hz under the hood, but cutoff is a Phonon pattern!
~lfo: sine 0.1 * 1000 + 500
out: saw 110 # moogLadder ~lfo 0.8
```

### Q: What about UGens fundsp doesn't have?

**A**: We implement those custom (30 UGens):
- Analysis (pitchTrack, onsetDetect) - use realfft crate
- Granular synthesis - implement from papers
- Physical modeling (waveguide) - Julius O. Smith's work
- Spatial audio (binaural) - use rust-spatial-audio or custom

### Q: Will this change Phonon's syntax?

**A**: No! Syntax stays exactly the same:
- Space-separated parameters: `lpf 1000 0.8`
- `#` for chaining: `saw 220 # lpf 1000 0.8`
- Pattern modulation: `~lfo: sine 0.1` then `lpf ~lfo 0.8`

The fundsp integration is **internal** - users don't see it.

### Q: How do we handle fundsp's graph notation (`>>`, `|`, etc.)?

**A**: We **don't expose it** to users. Phonon has its own syntax (`#`, `$`, `+`, `*`). fundsp's graph notation is used internally when creating units, but users never see it:

```rust
// Internal: We use fundsp's graph notation
let unit = lowpass_hz(1000.0, 1.0) >> highpass_hz(100.0, 1.0);

// User: Phonon syntax
"~bass: saw 55 # lpf 1000 1.0 # hpf 100 1.0"
```

### Q: Performance overhead of wrapping fundsp?

**A**: Minimal (<5%):
- fundsp uses SIMD internally
- Our wrapper just evaluates Phonon signals then calls fundsp
- Pattern evaluation is already optimized
- In practice, fundsp is often **faster** than our custom DSP

### Q: Can we mix fundsp and custom UGens?

**A**: Yes! Example:

```phonon
-- Custom sample playback
~drums: s "bd sn hh cp"

-- fundsp filter
~filtered: ~drums # moogLadder 2000 0.7

-- Custom reverb (could also use fundsp::reverb_stereo)
~wet: ~filtered # reverb 0.3

out: ~wet
```

Users don't know or care which UGens are fundsp vs custom.

---

## Next Steps

**This Week** (Week 1):
1. ✅ Complete this design document
2. Implement `SignalNode::FundspUnit` wrapper
3. Test pattern modulation with fundsp units
4. Get 5 oscillators working (sine, saw, square, triangle, pulse)

**Next Week** (Week 2):
5. Implement compiler functions for all fundsp oscillators
6. Add 3-level tests for each
7. Update UGEN_IMPLEMENTATION_CHECKLIST.md

**Month 1 Goal**:
- All fundsp oscillators wrapped (20 UGens)
- All tests passing
- 10+ musical examples

**Quarter 1 Goal**:
- 60 fundsp UGens wrapped
- 10 custom UGens implemented
- 70/90 UGens complete (78% parity)

**Year 1 Goal**:
- 90/90 UGens complete (100% SuperCollider parity)
- 270+ tests passing
- Production-ready synthesis system

---

*Let's build the synthesis system Rust deserves, standing on the shoulders of giants.*

**Status**: Design Complete - Ready for Implementation
**Next Action**: Implement SignalNode::FundspUnit wrapper (Week 1, Day 1)
