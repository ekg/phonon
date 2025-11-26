# THE VISION

**Phonon is an expressive medium that transforms thoughts into sound design and cool beats.**

Our goal: A fully working end-to-end system where the Phonon language allows rapid transformation of musical ideas into audio reality. Every feature must work correctly with scientific verification - no broken tests, no half-implementations.

---

## 🚨 CRITICAL ARCHITECTURAL RULE 🚨

**EVERY PARAMETER MUST BE A PATTERN**

```rust
// ❌ WRONG - bare types not allowed:
pub fn swing(self, amount: f64) -> Self

// ✅ CORRECT - all parameters are patterns:
pub fn swing(self, amount: Pattern<f64>) -> Self
where T: Clone + Send + Sync + 'static
```

**Why**: Pattern-controlled parameters are Phonon's superpower. Unlike Tidal/Strudel where patterns only trigger discrete events, in Phonon patterns modulate ANY parameter in real-time.

**Enforcement**: The compiler wraps constants with `Pattern::pure()`, so `fast 2` and `fast "2 3 4"` both work. Methods MUST accept `Pattern<T>`, never bare types.

---

## CURRENT STATUS (2025-11-25)

### Test Suite: 1837 tests passing ✅

### Recent Accomplishments

**Tidal Time Functions (Complete)**:
- ✅ `rotL` / `rotR` - Fixed to match Tidal semantics (shift query + results)
- ✅ `fastGap` - Compress pattern with gap
- ✅ `zoom` - Extract time span, stretch to cycle
- ✅ `press` / `pressBy` - Delay by slot fraction
- ✅ `ghost` / `ghostWith` - Add ghost notes
- ✅ `swing` - Shuffle timing (already working)
- ✅ `inside` / `outside` - Time scale operations (already working)
- ✅ `within` - Apply function to time range (already working)

**Live Coding Improvements**:
- ✅ Ring buffer clear on graph swap (instant C-x transitions)
- ✅ Reduced refresh rate (100ms, less CPU)
- ✅ Fixed underrun display (doesn't get stuck in red)
- ✅ More detailed performance info

**Time Control**:
- ✅ `tempo:` / `bpm:` - Static tempo
- ✅ `setCycle` / `resetCycles` / `nudge` - Time manipulation

### Working Features
- ✅ Voice-based sample playback (64 voices, polyphonic)
- ✅ **Sample bank selection**: `s "bd:0 bd:1 bd:2"` (select specific samples from folders)
- ✅ Pattern transformations: `fast`, `slow`, `rev`, `every`, `rotL/R`, etc.
- ✅ Signal flow chaining: `#` (left to right)
- ✅ Pattern-controlled synthesis: `sine "110 220 440"`
- ✅ Pattern-controlled filters: `saw 55 # lpf "500 2000" 0.8`
- ✅ Sample routing through effects: `s "bd sn" # lpf 2000 0.8`
- ✅ Live coding with phonon-edit (instant transitions)
- ✅ Mini-notation: Euclidean, alternation, subdivision, rests
- ✅ Comment support with `--` (double-dash)

### Next Priority Features

**High Value Quick Wins**:
1. Pattern DSP parameters (`gain`, `pan`, `speed` as patterns)
2. More time functions (`hurry`, `chop`, `striate`, `loopAt`)

**Architectural**:
1. DAW-style buffer passing (block-based, not sample-by-sample)
2. Multi-output system (`out1:`, `out2:`, `hush`, `panic`)

**UGen Development** (see docs/SYNTHESIS_PARITY_PLAN.md):
- Next: FM oscillator, white noise, pulse/PWM, Pan2

---

## CORE DEVELOPMENT PRINCIPLES

1. **Test-Driven Development (TDD)** - MANDATORY for all features
2. **Never say "next steps"** - just start working on them
3. **Research before implementing** - find working examples first
4. **Audio testing** - use signal analysis, not just compilation
5. **COMPLETE THE WORK** - no half-measures, deliver the full vision

---

## TDD WORKFLOW (REQUIRED)

**For every feature:**

1. **Write failing test** - demonstrate desired behavior
2. **Run test** - confirm it fails
3. **Implement** - minimal code to pass
4. **Run test** - confirm it passes
5. **Refactor** - keep tests passing
6. **Commit** - with descriptive message

---

## AUDIO TESTING - THREE-LEVEL METHODOLOGY

**EVERY pattern/audio feature needs all three levels:**

### Level 1: Pattern Query Verification
Fast, exact, deterministic - tests pattern logic without rendering audio.

```rust
let pattern = parse_mini_notation("bd sn hh cp");
let fast2 = pattern.fast(Pattern::pure(2.0));

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

**Catches**: Pattern logic bugs, incorrect event counts, cycle boundary issues

### Level 2: Onset Detection
Tests that audio events occur at the right times.

```rust
use pattern_verification_utils::detect_audio_events;

let audio = render_dsl(code, duration);
let onsets = detect_audio_events(&audio, 44100.0, threshold);

assert_eq!(onsets.len(), expected_event_count);
```

**Catches**: Silent audio, wrong timing, missing events, doubled events

### Level 3: Audio Characteristics
Sanity check on signal quality.

```rust
let rms = calculate_rms(&audio);
assert!(rms > 0.01); // Has sound
assert!(rms_fast > rms_normal); // More events = more energy
```

**Catches**: Complete silence, unexpected amplitude, clipping

---

## CRITICAL SYNTHESIS RULE

**BEFORE attempting ANY synthesis/audio work:**

1. **Research HOW it works** in the codebase first
2. **Find WORKING examples** and test them
3. **Never guess at synthesis** - always test
4. **Use SAMPLES (dirt-samples)** not sine-wave synthesis for drums

---

## DSL SYNTAX

**SPACE-SEPARATED ONLY** (optimized for live coding):
- ✅ CORRECT: `lpf 1000 0.8`
- ❌ WRONG: `lpf(1000, 0.8)`

Parentheses and commas are NOT supported for function calls.

### Bus Syntax

**New Syntax (recommended)**: Uses `$` for audio sources, `#` for modifier/parameter buses:
- `$` - assigns audio source OR applies pattern transform
- `#` - creates modifier/parameter bus (LFOs, control patterns)

```phonon
-- Comments use double-dash
cps: 2.0

-- Audio buses with $ (signal generators)
~drums $ s "bd sn hh*4 cp"
~bass $ saw "55 82.5"

-- Modifier buses with # (parameter control)
~lfo # sine 2                      -- LFO for modulation
~cutoff # "500 1000 2000 1500"     -- stepped parameter pattern

-- Using modifier buses in audio chain
~filtered $ saw 55 # lpf (~lfo * 500 + 800) 0.8
~stepped $ saw 110 # lpf ~cutoff 0.7

-- Sample bank selection with :N syntax
~kicks $ s "bd:0 bd:1 bd:2"

-- Pattern transformations ($ chains transforms)
~fast_drums $ s "bd sn" $ fast 2
~reversed $ s "bd sn" $ rev

-- Output ($ assigns)
out $ ~drums * 0.4 + ~bass * 0.3
```

**Legacy syntax**: Colon `:` still works for backward compatibility:
```phonon
~bass: saw 55          -- Still valid
out: ~bass * 0.3       -- Still valid
```

---

## UGEN IMPLEMENTATION STRATEGY

**Vision**: Achieve full CSound/SuperCollider feature parity while maintaining Phonon's live-coding elegance.

### The Approach: Don't Reinvent, Integrate and Learn

1. **Rust Audio Ecosystem** - `fundsp`, `biquad`, `rubato` (production-ready)
2. **SuperCollider** - 30+ years of synthesis research (study, port with attribution)
3. **CSound** - Comprehensive opcode library (learn from, implement cleanly)
4. **Academic Papers** - Julius O. Smith, Will Pirkle, Udo Zölzer

### Key Documents

- **docs/SYNTHESIS_PARITY_PLAN.md** - Complete 90-UGen roadmap
- **docs/UGEN_IMPLEMENTATION_GUIDE.md** - Step-by-step guide (~2 hours per UGen)
- **docs/UGEN_STATUS.md** - Progress tracker

### UGen Workflow (TDD)

1. **Write test FIRST** (30 min) - all three levels
2. **Run test** - confirm fails
3. **Implement UGen** (1 hour)
   - Define in `src/unified_graph.rs`
   - Add evaluation in `eval_node()`
   - Add compiler in `src/compositional_compiler.rs`
   - Register in function table
4. **Run test** - confirm passes
5. **Create musical example** (10 min)
6. **Commit**

**Timeline**: ~2 hours per UGen with comprehensive tests

### Essential UGens (Next Priority)
- FM oscillator
- White noise
- Pulse/PWM
- Pan2 (stereo)
- Limiter
- Parametric EQ

---

## AUTONOMOUS WORK DIRECTIVE

**When given permission to work autonomously:**

- **DO NOT STOP** to ask permission for each step
- **DO NOT PAUSE** after completing individual todos
- **CONTINUE WORKING** through the entire task list
- **TEST COMPREHENSIVELY** - don't just make it compile, make it work
- **DELIVER A WORKING SYSTEM** - not a partial implementation

**The answer is ALWAYS: continue until complete.**

---

## KEY FILES TO MODIFY

**Parser (DSL syntax)**:
- `src/main.rs` - Main parser
- `src/unified_graph_parser.rs` - Expression parsing

**Audio Engine**:
- `src/unified_graph.rs` - Signal graph evaluation
- `src/voice_manager.rs` - Polyphonic sample playback
- `src/sample_loader.rs` - Sample bank loading

**Pattern System**:
- `src/pattern.rs` - Core pattern types & transforms
- `src/pattern_ops.rs` - Pattern transformations
- `src/pattern_ops_extended.rs` - Extended transforms
- `src/pattern_structure.rs` - Structure operations
- `src/mini_notation_v3.rs` - Mini-notation parsing

**Testing**:
- `tests/` - Integration tests
- `src/bin/wav_analyze.rs` - Audio analysis tool

---

## WHAT MAKES PHONON UNIQUE

**Patterns ARE control signals** - evaluated at sample rate (44.1kHz)

```phonon
-- This is IMPOSSIBLE in Tidal/Strudel:
~lfo $ sine 0.25                          -- Pattern as LFO
out $ saw "55 82.5" # lpf (~lfo * 2000 + 500) 0.8
-- Pattern modulates filter cutoff continuously!
```

In Tidal/Strudel, patterns only trigger discrete events. In Phonon, patterns can modulate any synthesis parameter in real-time.
