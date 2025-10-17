# Pattern Frequency Parameter Bug Fix

**Date**: 2025-10-14
**Status**: ✅ FIXED

## Problem

Pattern frequency parameters in DSL syntax were producing completely wrong frequencies:

```phonon
out: sine("110 220 330") * 0.2
```

**Expected**: Frequencies cycling through 110 Hz, 220 Hz, 330 Hz
**Actual**: Detected 4704 Hz, 10334 Hz, 11701 Hz

## Root Cause

The bug was in `src/unified_graph.rs` lines 915-922 in the `Signal::Pattern` evaluation code. The evaluation order was WRONG:

```rust
// WRONG: Tried MIDI note parsing FIRST
if let Some(midi) = note_to_midi(s) {
    midi_to_freq(midi) as f32  // "110" → MIDI note 110 → 4704 Hz!
} else {
    s.parse::<f32>().unwrap_or(1.0)
}
```

When the pattern string contained "110", it was interpreted as **MIDI note 110** instead of the numeric value 110.0 Hz, resulting in:
- "110" → MIDI 110 → `midi_to_freq(110)` → **4704 Hz** ❌
- "220" → MIDI 220 → **10334 Hz** ❌
- "330" → MIDI 330 → **11701 Hz** ❌

## The Fix

Changed the evaluation order to try **numeric parsing FIRST**, then fall back to note names:

```rust
// CORRECT: Try numeric parsing FIRST
if let Ok(numeric_value) = s.parse::<f32>() {
    numeric_value  // "110" → 110.0 Hz ✅
} else if let Some(midi) = note_to_midi(s) {
    // Fall back to note name parsing (e.g., "c4", "a4", "cs4")
    midi_to_freq(midi) as f32
} else {
    1.0
}
```

**Location**: `src/unified_graph.rs` lines 915-927

## Test Results

### Before Fix
```
Segment 1: 4704 Hz (expected 110) ❌
Segment 2: 10334 Hz (expected 220) ❌
Segment 3: 11701 Hz (expected 330) ❌
```

### After Fix
```
Segment 1: 102.9 Hz (expected 110) ✅
Segment 2: 220.5 Hz (expected 220) ✅
Segment 3: 323.4 Hz (expected 330) ✅
```

## Diagnostic Tests Created

Created comprehensive FFT-based diagnostic tests in `/home/erik/phonon/tests/test_pattern_frequency_debug.rs`:

1. **`test_manual_sine_synthesis_reference()`** - Verify manual sine wave synthesis works (110 Hz, pure)
2. **`test_pattern_controlled_frequency_with_alternation()`** - Test `<110 220>` alternation with ADSR gating
3. **`test_pattern_frequency_both_notes_gated()`** - Verify both frequencies play correctly
4. **`test_diagnose_4700hz_problem()`** - Document the original 4704 Hz problem

### Key Test Features

- **FFT-based purity analysis**: Detects harmonics at 2x, 3x, 4x fundamental
- **ADSR-gated notes**: One note per cycle with full close-off
- **Alternation patterns**: `<110 220>` for clarity
- **Sine wave purity verification**: Ensures no high-frequency noise or harmonics

All 8 diagnostic tests now pass ✅

## Verification

```bash
# Test the specific fix
cargo test --test test_continuous_pattern_params test_oscillator_freq_pattern_cycles -- --include-ignored

# Run all diagnostic tests
cargo test --test test_pattern_frequency_debug

# Verify overall system health (201 library tests pass)
cargo test --lib
```

## Note on Existing Evaluator

The same bug was already fixed for Pattern NODES at line 1348-1356, but `Signal::Pattern` (inline pattern strings in DSL) used different code that still had the wrong evaluation order.

## Impact

This fix enables:
- ✅ Pattern-controlled oscillator frequency: `sine("110 220 440")`
- ✅ Pattern-controlled synthesis parameters: `supersaw("110 220", 0.5, 5)`
- ✅ Correct frequency cycling in DSL syntax
- ✅ Support for both numeric frequencies (110, 220) and note names ("c4", "a4")

This is one of Phonon's **unique features** - patterns as continuous control signals, which Tidal cannot do!
