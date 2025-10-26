# THE VISION

**Phonon is an expressive medium that transforms thoughts into sound design and cool beats.**

Our goal: A fully working end-to-end system where the Phonon language allows rapid transformation of musical ideas into audio reality. Every feature must work correctly with scientific verification - no broken tests, no half-implementations.

## CURRENT SESSION CONTEXT (2025-10-25)

**CURRENT PHASE: AUDIO-BASED TEST DEVELOPMENT**

We are in a critical testing and verification phase. Recent discoveries revealed:
- ✅ **Two critical bugs fixed**: Transforms not applying, duplicate event triggers
- ⚠️ **Test suite inadequate**: Tests only verify compilation, not correctness
- ✅ **40+ transforms verified**: Systematic audio analysis confirms they work
- 🎯 **Next focus**: Design and implement adequate audio-based tests

**The Challenge**: Testing audio systems requires computational audio analysis, not just checking if code compiles.

**The Opportunity**: We have the tools and knowledge to do proper audio verification.

**NEVER ASK "Should we continue?" - The answer is ALWAYS YES**

## ⚠️ CRITICAL DIRECTIVE ⚠️

**DO NOT STOP UNTIL THE VISION IS COMPLETE**

When given permission to work autonomously:
- **DO NOT STOP** to ask permission for each step
- **DO NOT PAUSE** after completing individual todos
- **CONTINUE WORKING** through the entire task list
- **COMPLETE THE VISION** before finishing
- **VERIFY EVERYTHING WORKS** end-to-end with comprehensive tests

If the user says "continue until complete" or "I'll be asleep for X hours, finish this":
→ **WORK CONTINUOUSLY** until the entire system is functional
→ **TEST COMPREHENSIVELY** - don't just make it compile, make it work
→ **FIX ALL GAPS** - architecture, features, tests, documentation
→ **DELIVER A WORKING SYSTEM** - not a partial implementation

## Core Development Principles

1. **Test-Driven Development (TDD)** - MANDATORY for all features
2. **Never say "next steps"** - just start working on them
3. **Research before implementing** - find working examples first
4. **Audio testing** - use signal analysis, not just compilation
5. **COMPLETE THE WORK** - no half-measures, deliver the full vision

## TDD Workflow (REQUIRED FOR ALL FEATURES)

**For every single feature, follow this exact workflow:**

1. **Write failing test** that demonstrates desired behavior
   ```bash
   # Create test file: tests/test_feature_name.rs
   # Test should clearly show what the feature SHOULD do
   ```

2. **Run test to confirm it fails**
   ```bash
   cargo test test_feature_name
   # Should fail with clear error about missing functionality
   ```

3. **Implement minimal code** to make test pass
   ```rust
   // Add to src/main.rs, src/unified_graph.rs, etc.
   // Only write what's needed to pass the test
   ```

4. **Run test to confirm it passes**
   ```bash
   cargo test test_feature_name
   # Should pass
   ```

5. **Refactor if needed** (keep tests passing)

6. **Commit with descriptive message**
   ```bash
   git add tests/test_feature_name.rs src/main.rs
   git commit -m "Implement feature_name with tests"
   ```

## Audio Testing Guidelines - MANDATORY

### Why Audio-Based Testing is Critical

**THE PROBLEM**: Tests that only check compilation or RMS hide critical bugs.
- ❌ `assert_eq!(buffer.len(), 44100)` - Only verifies buffer exists
- ❌ `assert!(rms > 0.01)` - Only verifies "some sound" exists
- ✅ **Three-Level Verification** - Catches bugs other approaches miss

**Critical Bugs Found Through Multi-Level Testing**:
1. Pattern transforms silently ignored (transforms compiled but didn't apply)
2. Duplicate events on cycle boundaries (events triggered twice)
3. Sample playback completely silent in render mode (found via onset detection)

### The Three-Level Testing Methodology

**EVERY test for pattern/audio features must use all three levels:**

#### LEVEL 1: Pattern Query Verification (Exact, Fast, Deterministic)

Tests the **pattern logic** without rendering audio. This is:
- ✅ Fast (no audio rendering)
- ✅ Exact (precise event counts)
- ✅ Deterministic (no timing variability)

```rust
// Test pattern produces correct events
let pattern = parse_mini_notation("bd sn hh cp");
let fast2 = pattern.fast(2.0);

let mut total = 0;
for cycle in 0..8 {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    };
    total += fast2.query(&state).len();
}
assert_eq!(total, 64); // 4 events × 2 (fast) × 8 cycles
```

**What This Catches**: Pattern logic bugs, incorrect event counts, cycle boundary issues

#### LEVEL 2: Onset Detection (Audio Timing Verification)

Tests that audio **events actually occur** at the right times. Uses onset detection to find peaks/transients.

```rust
use pattern_verification_utils::detect_audio_events;

let audio = render_dsl(code, duration);
let onsets = detect_audio_events(&audio, 44100.0, threshold);

// Verify onset count matches pattern
assert_eq!(onsets.len(), expected_event_count);

// Verify timing/intervals
if onsets.len() >= 2 {
    let interval = onsets[1].time - onsets[0].time;
    assert!((interval - expected_interval).abs() < tolerance);
}
```

**What This Catches**: Silent audio, wrong timing, missing events, doubled events

**Why RMS Alone Fails**:
- `fast 9` with wrong timing: RMS looks normal, but events at wrong times
- `rev` (reverse): RMS identical, but order wrong (onset times reveal this)
- Silent output: RMS = 0, but doesn't tell you *why*

#### LEVEL 3: Audio Characteristics (Signal Quality)

Tests overall audio properties as a sanity check.

```rust
let rms = calculate_rms(&audio);
assert!(rms > 0.01); // Has sound
assert!(rms_fast > rms_normal); // More events = more energy
```

**What This Catches**: Complete silence, unexpected amplitude, clipping

### Test Requirements - EVERY Feature MUST Have All Three

For pattern operations (fast, slow, rev, etc.):
1. **Pattern queries** over 4-8 cycles verifying event count/structure
2. **Onset detection** verifying audio events match expected timing
3. **RMS/amplitude** as final sanity check

### Testing Tools

**Event Counting** (pattern correctness):
```rust
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};

let pattern = parse_mini_notation("bd sn").fast(2.0);
for cycle in 0..8 {
    let state = State {
        span: TimeSpan::new(
            Fraction::from_float(cycle as f64),
            Fraction::from_float((cycle + 1) as f64),
        ),
        controls: HashMap::new(),
    };
    let events = pattern.query(&state);
    assert_eq!(events.len(), expected);
}
```

**Audio Analysis** (signal verification):
```bash
cargo run --bin wav_analyze -- output.wav
# Look for: RMS level, Peak level, frequency content
```

**Render and Measure** (integration testing):
```rust
fn test_transform_doubles_density() {
    let normal = render("s \"bd*4\"", 8);  // 8 cycles
    let fast = render("s \"bd*4\" $ fast 2", 8);

    // Event count verification
    assert_eq!(normal.event_count, 32);
    assert_eq!(fast.event_count, 64);

    // Audio verification
    assert!(fast.rms > normal.rms * 1.4);  // More events = higher energy
}
```

### Ensuring Correct Expectations

**CRITICAL**: Your test expectations MUST be correct!

Before writing a test:
1. **Research expected behavior** - check Tidal Cycles documentation
2. **Manual verification** - render audio and listen/analyze
3. **Event counting** - query pattern to see actual events
4. **Cross-reference** - compare with known working examples

**Example of verifying expectations**:
```rust
// WRONG: Assuming without verification
assert_eq!(pattern.query().len(), 10);  // Where did 10 come from?

// RIGHT: Derived from first principles
let base_events = 4;  // "bd sn hh cp" = 4 events/cycle
let cycles = 8;
let fast_factor = 2.0;
let expected = base_events * cycles * fast_factor as usize;  // = 64
assert_eq!(total_events, expected);
```

### Test Template

```rust
#[test]
fn test_transform_name_behavior() {
    // 1. Define expectations from first principles
    let base_pattern = "bd sn hh cp";  // 4 events/cycle
    let cycles = 8;
    let expected_normal = 4 * cycles;  // 32 events
    let expected_transformed = expected_normal * 2;  // Depends on transform

    // 2. Query pattern events
    let pattern = parse_mini_notation(base_pattern).transform();
    let mut actual = 0;
    for cycle in 0..cycles {
        actual += count_events_in_cycle(pattern, cycle);
    }

    // 3. Verify event count
    assert_eq!(actual, expected_transformed,
        "Transform should produce {} events over {} cycles",
        expected_transformed, cycles);

    // 4. Render and verify audio
    let audio = render_pattern(pattern, cycles);
    assert!(audio.rms > threshold, "Audio energy too low");

    // 5. Compare to baseline
    let baseline = render_pattern(parse_mini_notation(base_pattern), cycles);
    verify_relationship(audio, baseline);
}
```

### Multi-Cycle Testing Requirements

**ALWAYS test over multiple cycles** (4-8 minimum):
- Catch cycle boundary bugs (like duplicate event triggers)
- Verify pattern consistency across cycles
- Test conditional transforms (`every n`)
- Observe progressive changes (`iter`, `palindrome`)

**Example**:
```rust
// BAD: Only tests 1 cycle
let buffer = render(code, 1.0);
assert_eq!(buffer.len(), 44100);

// GOOD: Tests 8 cycles with verification
for cycle in 0..8 {
    let events = query_cycle(pattern, cycle);
    assert_eq!(events.len(), expected_per_cycle);
    verify_event_content(events);
}
```

## Current Status (2025-10-11)

### Working Features (182 tests passing)
- ✅ Voice-based sample playback (64 voices, polyphonic)
- ✅ Pattern transformations: `$` with `fast`, `slow`, `rev`, `every`
- ✅ Signal flow chaining: `#` (left to right)
- ✅ Pattern-controlled synthesis: `sine "110 220 440"`
- ✅ Pattern-controlled filters: `saw 55 # lpf "500 2000" 0.8`
- ✅ Sample routing through effects: `s "bd sn" # lpf 2000 0.8`
- ✅ Live coding with auto-reload
- ✅ Mini-notation: Euclidean, alternation, subdivision, rests
- ✅ --cycles parameter correctly accounts for tempo
- ✅ Comment support with `--` (double-dash)

### Next Priority Features (See ROADMAP.md)
1. Multi-output system (`out1:`, `out2:`, etc. + `hush` + `panic`)
2. Sample bank selection (`s "bd:0 bd:1"`)
3. Pattern DSP parameters (`gain`, `pan`, `speed`)

**See docs/ROADMAP.md for complete feature list and implementation plan**

## UGEN IMPLEMENTATION STRATEGY - PATH TO PARITY

**Vision**: Achieve full CSound/SuperCollider feature parity while maintaining Phonon's live-coding elegance.

**Timeline**: 18-24 months to 90 UGens (oscillators, filters, envelopes, effects, analysis)

### The Approach: Don't Reinvent, **Integrate and Learn**

We stand on the shoulders of giants:

1. **Rust Audio Ecosystem** - `fundsp`, `biquad`, `rubato` (production-ready implementations)
2. **SuperCollider** - 30+ years of synthesis research (study algorithms, port with attribution)
3. **CSound** - Comprehensive opcode library (learn from, implement cleanly)
4. **Academic Papers** - Julius O. Smith, Will Pirkle, Udo Zölzer (authoritative sources)
5. **Our Architecture** - Already solid, just add `SignalNode` variants systematically

### Key Documents (MUST READ before implementing UGens)

- **docs/SYNTHESIS_PARITY_PLAN.md** - Complete 90-UGen roadmap with porting strategies
- **docs/UGEN_IMPLEMENTATION_GUIDE.md** - Step-by-step guide (adds UGen in ~2 hours)
- **docs/UGEN_STATUS.md** - Progress tracker, shows what's done/in-progress/planned

### Implementation Workflow (TDD - MANDATORY)

For **every single UGen**, follow this exact sequence:

#### 1. Write Test FIRST (30 min)
```rust
// tests/test_adsr_envelope.rs
#[test]
fn test_adsr_level1_pattern_query() {
    // LEVEL 1: Pattern logic verification
    let pattern = parse_mini_notation("x ~ x ~");
    // Test event counts over cycles
}

#[test]
fn test_adsr_level2_onset_detection() {
    // LEVEL 2: Audio event verification
    let audio = render_dsl("~env: trigger # adsr 0.01 0.1 0.5 0.2");
    let onsets = detect_audio_events(&audio);
    // Verify envelope triggers
}

#[test]
fn test_adsr_level3_envelope_shape() {
    // LEVEL 3: Signal quality verification
    let audio = render_dsl("~env: x # adsr 0.1 0.2 0.5 0.3");
    // Verify attack/decay/sustain/release shape
}
```

#### 2. Run Test - Confirm it FAILS (2 min)
```bash
cargo test test_adsr
# Should error: "Unknown function: adsr"
```

#### 3. Implement UGen (1 hour)

**Step 3a**: Define in `src/unified_graph.rs`
```rust
SignalNode::ADSR {
    trigger: Signal,
    attack: Signal,
    decay: Signal,
    sustain: Signal,
    release: Signal,
    state: ADSRState,
}
```

**Step 3b**: Add evaluation logic in `eval_node()`
```rust
SignalNode::ADSR { trigger, attack, decay, sustain, release, state } => {
    // Implement ADSR algorithm
    // Return envelope value (0.0 to 1.0)
}
```

**Step 3c**: Add compiler in `src/compositional_compiler.rs`
```rust
fn compile_adsr(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Parse parameters, create node
}
```

**Step 3d**: Register in function table
```rust
"adsr" => compile_adsr(ctx, args),
```

#### 4. Run Test - Confirm it PASSES (2 min)
```bash
cargo test test_adsr
# All 3 levels should pass
```

#### 5. Create Musical Example (10 min)
```phonon
-- docs/examples/adsr_demo.ph
tempo: 2.0
~trigger: "x ~ x ~"
~env: ~trigger # adsr 0.01 0.1 0.7 0.2
~synth: sine 440 * ~env
out: ~synth
```

#### 6. Commit (2 min)
```bash
git add tests/test_adsr_envelope.rs src/unified_graph.rs src/compositional_compiler.rs docs/examples/adsr_demo.ph
git commit -m "Implement ADSR envelope with three-level tests

- Pattern-triggered ADSR envelope generator
- Attack, decay, sustain, release parameters
- All pattern-controllable
- Tests: pattern query, onset detection, envelope shape
- Example: adsr_demo.ph"
```

### Prioritized Implementation Order

**Tier 1 - Essential (3 months, 10 UGens)**:
1. ✅ ADSR envelope (week 1) - Enables 80% of synth patches
2. FM oscillator (week 2) - Huge range of sounds
3. White noise (week 3) - Drums/percussion
4. Pulse/PWM (week 4) - Analog synth sounds
5. Pan2 (week 5-6) - Stereo output
6. Limiter (week 7) - Production-ready mixes
7. Parametric EQ (week 8) - Sound shaping
8. Moog ladder filter (week 9) - Classic analog
9. Ring modulator (week 10) - Metallic sounds
10. Flanger (week 11) - Modulation effect

**Result**: "Phonon can make professional tracks"

**Tier 2 - Advanced (6 months, 20 UGens)**:
- Granular synthesis, wavetable, physical modeling
- Convolution reverb, vocoder, pitch shifter
- Advanced filters, multi-band processing

**Tier 3 - Specialized (6 months, 10 UGens)**:
- Binaural/HRTF, ambisonics, spatial audio
- Advanced phase vocoder operations

### Key Principles for UGen Development

1. **Test-Driven Development (TDD)** - Write failing test first, always
2. **Three-Level Verification** - Pattern query, onset detection, audio quality
3. **Research First** - Study existing implementations before coding
4. **Musical Examples** - Every UGen needs a demo showing musical use
5. **Clean Code** - Follow the template in UGEN_IMPLEMENTATION_GUIDE.md
6. **Attribution** - Cite sources in comments (papers, SC code, etc.)
7. **Licensing** - Track provenance (MIT for original, GPL for ported SC code)

### Porting from SuperCollider

**Legal/Ethical Approach**:

```rust
/// FM Synthesis implementation
///
/// Algorithm based on:
/// - John Chowning (1973) "The Synthesis of Complex Audio Spectra"
/// - SuperCollider's FM7.cpp (GPL) - studied for reference
///
/// This is a clean-room reimplementation based on the original
/// academic paper and understanding of the SC source code.
///
/// License: MIT (clean-room) or GPL (if directly ported)
SignalNode::FM { ... }
```

**Process**:
1. Read SC source to understand algorithm
2. Read original academic paper
3. Implement from first principles in Rust
4. Test against SC output for correctness
5. Document sources and approach

### Integration with Rust Ecosystem

**Use Existing Crates Where Possible**:

```rust
// Example: Integrate fundsp oscillator
use fundsp::prelude::*;

SignalNode::FunDSPSaw {
    frequency: Signal,
    oscillator: Box<dyn AudioUnit>,
}
```

**Available Resources**:
- `fundsp` - Complete DSP framework (50+ UGens already!)
- `biquad` - Production-ready filters
- `rubato` - Sample rate conversion
- `realfft` - FFT operations for spectral processing

### Progress Tracking

**After Each UGen**:
1. Update `docs/UGEN_STATUS.md` - Mark ✅ complete
2. Run full test suite - Ensure nothing broke
3. Add to `docs/SYNTHESIS_PARITY_PLAN.md` weekly tracker
4. Share example in Discord/forum for feedback

### Success Metrics

**Technical**:
- 340+ tests passing (currently)
- Target: 500+ tests (all three levels for each UGen)
- Real-time performance on consumer hardware

**Musical**:
- Can make professional techno/house tracks
- Can create realistic instruments
- Can do experimental/avant-garde sound design
- Used in live performances

### Current Status (2025-10-25)

**UGens Implemented**: 10/90 (11%)
- ✅ Oscillators: sine, saw, square, triangle
- ✅ Filters: lpf, hpf, bpf
- ✅ Effects: reverb, delay, distortion, chorus, compressor, bitcrush

**Next to Implement** (Starting NOW):
- ADSR envelope (this session)
- FM oscillator (next)
- White noise (next)

**Why This Will Work**:
- Architecture is solid (just add nodes)
- Testing methodology catches bugs early
- Standing on 50 years of DSP research
- Rust ecosystem has production-ready components
- Community can contribute (clear templates)

**Timeline to Parity**: 18-24 months at 1 UGen/week, faster with contributors

## CRITICAL SYNTHESIS RULE

**BEFORE attempting ANY synthesis/audio work:**

1. **Research HOW it works** in the codebase first
2. **Find WORKING examples** and test them
3. **Never guess at synthesis** - always test
4. **Use SAMPLES (dirt-samples)** not sine-wave synthesis for drums
5. **Sample triggering syntax** must be researched, not invented

## Implementation Strategy

When implementing features from ROADMAP.md:

1. **Read ROADMAP.md** - understand the feature requirements
2. **Write test FIRST** - before any implementation
3. **Run test** - confirm it fails
4. **Implement minimally** - just enough to pass test
5. **Verify** - test passes
6. **Document** - update ROADMAP.md to mark feature complete
7. **Commit** - with clear message referencing test

### Key Files to Modify

**Parser (DSL syntax)**:
- `src/main.rs` - Main parser for render and live modes
- `src/unified_graph_parser.rs` - Expression parsing

**Audio Engine**:
- `src/unified_graph.rs` - Signal graph evaluation
- `src/voice_manager.rs` - Polyphonic sample playback
- `src/sample_loader.rs` - Sample bank loading

**Pattern System**:
- `src/pattern.rs` - Core pattern types
- `src/pattern_ops.rs` - Pattern transformations
- `src/mini_notation_v3.rs` - Mini-notation parsing

**Testing**:
- `tests/` - Integration tests
- `src/bin/wav_analyze.rs` - Audio analysis tool

## Current DSL Syntax

**CRITICAL SYNTAX RULE: SPACE-SEPARATED ONLY**

Phonon uses **space-separated function syntax** ONLY. This is optimized for live coding:
- ✅ CORRECT: `lpf 1000 0.8`
- ❌ WRONG: `lpf(1000, 0.8)`

Parentheses and commas are NOT supported for function calls. This decision is deliberate:
- Fewer keystrokes = faster live coding
- Simpler syntax = fewer errors
- One way to do things = clear conventions

**Why this matters for AI agents:** Your training data overwhelmingly uses `function(arg1, arg2)` syntax. However, Phonon ONLY supports `function arg1 arg2`. You MUST actively override your training bias and use space-separated syntax.

```phonon
-- Comments use double-dash
tempo: 2.0              -- Cycles per second

-- Bus assignment
~lfo: sine 0.25
~bass: saw "55 82.5" # lpf (~lfo * 2000 + 500) 0.8

-- Sample playback
~drums: s "bd sn hh*4 cp"

-- Pattern transformations
~fast_drums: ~drums $ fast 2
~reversed: s "bd sn" $ rev

-- Signal flow (# chains left to right)
~filtered: s "bd sn" # lpf 2000 0.8

-- Output (required)
out: ~bass * 0.4 + ~drums * 0.6
```

## What Makes Phonon Unique

**Patterns ARE control signals** - evaluated at sample rate (44.1kHz)

```phonon
-- This is IMPOSSIBLE in Tidal/Strudel:
~lfo: sine 0.25                          -- Pattern as LFO
out: saw "55 82.5" # lpf (~lfo * 2000 + 500) 0.8
-- Pattern modulates filter cutoff continuously!
```

In Tidal/Strudel, patterns only trigger discrete events. In Phonon, patterns can modulate any synthesis parameter in real-time.