# THE VISION

**Phonon is an expressive medium that transforms thoughts into sound design and cool beats.**

Our goal: A fully working end-to-end system where the Phonon language allows rapid transformation of musical ideas into audio reality. Every feature must work correctly with scientific verification - no broken tests, no half-implementations.

## CURRENT SESSION CONTEXT (2025-10-18)

**AUTONOMOUS WORK MODE - 12 hours granted**
- User is away for 12 hours - continue working WITHOUT asking permission
- Fix ALL broken tests - no exceptions
- Current issue: 16 filter LFO/pattern modulation tests failing
- Recent achievement: Compressor effect fully implemented + fixed critical effects parameter bug
- Next: Fix filter modulation, then complete remaining e2e verification work

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

## Audio Testing Guidelines

Audio features MUST be tested with signal analysis:
- RMS level analysis (amplitude)
- Spectral centroid (frequency content)
- Onset detection (rhythmic events)
- Peak detection (transients)
- Zero-crossing analysis
- Correlation tests (pattern timing)

Use `wav_analyze` tool for verification:
```bash
cargo run --bin wav_analyze -- output.wav
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
- ✅ Comment support with `#` at start of line

### Next Priority Features (See ROADMAP.md)
1. Multi-output system (`out1:`, `out2:`, etc. + `hush` + `panic`)
2. Sample bank selection (`s "bd:0 bd:1"`)
3. Pattern DSP parameters (`gain`, `pan`, `speed`)

**See docs/ROADMAP.md for complete feature list and implementation plan**

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
# Comments
tempo: 2.0              # Cycles per second

# Bus assignment
~lfo: sine 0.25
~bass: saw "55 82.5" # lpf (~lfo * 2000 + 500) 0.8

# Sample playback
~drums: s "bd sn hh*4 cp"

# Pattern transformations
~fast_drums: ~drums $ fast 2
~reversed: s "bd sn" $ rev

# Signal flow (# chains left to right)
~filtered: s "bd sn" # lpf 2000 0.8

# Output (required)
out: ~bass * 0.4 + ~drums * 0.6
```

## What Makes Phonon Unique

**Patterns ARE control signals** - evaluated at sample rate (44.1kHz)

```phonon
# This is IMPOSSIBLE in Tidal/Strudel:
~lfo: sine 0.25                          # Pattern as LFO
out: saw "55 82.5" # lpf (~lfo * 2000 + 500) 0.8
# Pattern modulates filter cutoff continuously!
```

In Tidal/Strudel, patterns only trigger discrete events. In Phonon, patterns can modulate any synthesis parameter in real-time.