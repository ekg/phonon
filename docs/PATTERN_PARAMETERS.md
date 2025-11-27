# Pattern Parameters - Universal Modulation

**Status**: ✅ COMPLETE (as of 2025-11-12)

## Overview

**Every parameter in Phonon accepts patterns**, not just bare numbers. This is a fundamental design principle:

> **"Patterns ARE control signals"** - They can modulate any parameter at any rate, from discrete events to continuous sample-rate modulation.

## What This Means

```phonon
-- BEFORE (limited):
sine 440                    -- Fixed frequency
s "bd" # lpf 2000 0.8       -- Fixed cutoff

-- NOW (unlimited):
sine (sine 5 * 220 + 440)              -- LFO modulates frequency
s "bd" # lpf (sine 0.5 * 1500 + 500)   -- LFO modulates filter
s "bd*8" # ar "0.01 0.1" "0.1 0.5"     -- Pattern envelope
```

## Complete List: Everything That Accepts Patterns

### ✅ Oscillators (ALL frequencies)
```phonon
sine (sine 5 * 220 + 440)        -- Vibrato
saw (square 2 * 110 + 220)       -- Frequency modulation
square "110 220 440"             -- Pattern switching
triangle (sine 0.1 * 55 + 110)   -- Slow FM
```

### ✅ Filters (cutoff, Q, center)
```phonon
lpf (sine 0.5 * 1500 + 500) 0.8              -- Wah-wah
hpf (sine 1.0 * 1000 + 2000) "0.5 0.8 1.0"   -- Sweeping + Q pattern
bpf (sine 0.25 * 2000 + 1000) 4.0            -- Slow sweep, narrow
notch "500 1000 2000" 2.0                    -- Notch pattern
```

### ✅ Time-Based Effects
```phonon
-- Delay
delay (sine 1.0 * 0.2 + 0.1) 0.3           -- Modulated delay time
delay 0.25 (sine 2.0 * 0.3 + 0.5)          -- Modulated feedback
delay 0.125 0.6 (sine 1.0 * 0.5 + 0.5)     -- Modulated mix

-- Reverb
reverb (sine 0.25 * 0.5 + 0.3) 0.5         -- Room size modulation
reverb 0.8 (sine 0.5 * 0.5 + 0.3)          -- Damping modulation

-- Tape Delay
tapedelay (sine 1.0 * 0.3 + 0.2) 0.5       -- Vintage flutter
```

### ✅ Distortion & Saturation
```phonon
dist (sine 2.0 * 2.0 + 1.0)                -- Breathing distortion
bitcrush (sine 0.5 * 8 + 8)                -- Bit depth modulation
bitcrush 8 (sine 1.0 * 20000 + 22050)      -- Sample rate crush
```

### ✅ Modulation Effects
```phonon
-- Chorus
chorus (sine 0.5 * 2.0 + 3.0) 0.7          -- Rate modulation
chorus 2.0 (sine 1.0 * 0.5 + 0.5)          -- Depth modulation

-- Flanger
flanger (sine 0.25 * 0.5 + 0.5) 0.7        -- Slow sweep
flanger 0.5 (sine 2.0 * 0.5 + 0.5)         -- Depth modulation

-- Phaser
phaser (sine 0.5 * 1.0 + 1.0) 0.8          -- Rate modulation
phaser 1.0 (sine 1.0 * 0.5 + 0.5)          -- Feedback modulation

-- Tremolo
tremolo (sine 0.1 * 8 + 8) 0.7             -- Rate modulation
tremolo 5.0 (sine 0.25 * 0.5 + 0.5)        -- Depth modulation

-- Vibrato
vibrato (sine 0.5 * 5 + 5) 0.02            -- Rate modulation
vibrato 5.0 (sine 0.25 * 0.01 + 0.01)      -- Depth modulation
```

### ✅ Dynamics
```phonon
-- Compressor
compressor (sine 0.5 * -20 + -10) 4.0 0.01 0.1    -- Threshold modulation
compressor -12 (sine 1.0 * 4 + 4) 0.01 0.1        -- Ratio modulation

-- Amp/Gain
amp (sine 2.0 * 0.5 + 0.5)                         -- Tremolo
gain (sine 4.0 * 0.3 + 0.7)                        -- Amplitude modulation
```

### ✅ Sample Parameters
```phonon
s "bd*8" # gain "0.5 0.8 1.0 0.6"         -- Pattern volume
s "sn*4" # pan "-1 -0.5 0.5 1"            -- Pan pattern
s "hh*16" # speed "0.5 1.0 2.0"           -- Speed pattern
s "arpy" # ar "0.01 0.1" "0.1 0.5"        -- Envelope pattern
s "bd" # attack "0.001 0.01 0.1"          -- Attack pattern
s "sn" # release "0.1 0.3 0.8"            -- Release pattern
```

### ✅ Pattern Transforms
```phonon
-- Time transforms
s "bd*4" $ fast "2 3 4"                    -- Speed pattern
s "sn*2" $ slow "1 2"                      -- Slowdown pattern
s "cp*4" $ squeeze "2 3 4"                 -- Squeeze pattern
s "arpy" $ early "0.1 0.3"                 -- Early shift pattern
s "bass" $ late "0.1 0.3"                  -- Late shift pattern

-- Articulation
s "bd*8" $ legato "0.5 1.5"                -- Legato pattern
s "hh*8" $ staccato "0.1 0.8"              -- Staccato pattern
s "sn*4" $ swing "0.0 0.5"                 -- Swing pattern

-- Randomization
s "cp*8" $ degradeBy "0.1 0.9"             -- Dropout pattern
s "arpy*4" $ shuffle "0.1 0.9"             -- Shuffle pattern
```

## How It Works

### Implementation Strategy

All synthesis parameters use one of two approaches:

#### 1. **`compile_expr()` for effects** (most flexible)
```rust
let cutoff_node = compile_expr(ctx, cutoff_expr)?;
// compile_expr() supports:
// - Constants: 440
// - Patterns: "110 220 440"
// - Expressions: sine 0.5 * 1500 + 500
// - Buses: ~lfo
```

#### 2. **`.fmap()` for transform parameters** (pattern → f64)
```rust
// Pattern string → Pattern<String> → Pattern<f64>
let string_pattern = parse_mini_notation(pattern_str);
let factor_pattern = string_pattern.fmap(|s| s.parse::<f64>().unwrap_or(1.0));
```

### Signal Flow

```
User Code: sine (sine 5 * 220 + 440)
                    ↓
            compile_expr()
                    ↓
         Creates SignalNode tree:
            Oscillator {
              freq: Signal::Node(NodeId)  ← Points to LFO
            }
                    ↓
         Every sample, evaluate:
            LFO: sine 5 → -1.0 to 1.0
            Scale: * 220 → -220 to 220
            Offset: + 440 → 220 to 660
            Carrier: sine(frequency)
                    ↓
              Audio Out
```

## Musical Examples

### Classic Synthesis Techniques

**LFO Vibrato**:
```phonon
tempo: 0.5
~lfo: sine 5                              -- 5 Hz LFO
~vibrato: sine (~lfo * 10 + 440)          -- ±10 Hz vibrato around 440 Hz
out: ~vibrato
```

**Filter Sweep**:
```phonon
tempo: 1.0
~lfo: sine 0.25                           -- Slow 0.25 Hz LFO
~sweep: saw 110 # lpf (~lfo * 2000 + 500) 0.8   -- 500-2500 Hz sweep
out: ~sweep
```

**Rhythmic Pattern Modulation**:
```phonon
tempo: 0.5
~rhythm: s "bd*8"
~filtered: ~rhythm # lpf "500 1000 2000 4000 500 1000 2000 4000" 0.8
out: ~filtered
```

**Breathing Distortion**:
```phonon
tempo: 0.5
~lfo: sine 1.0                            -- 1 Hz breathing
~drive: ~lfo * 3.0 + 4.0                  -- Drive 1.0 to 7.0
~distorted: saw 110 # dist ~drive
out: ~distorted
```

## Why This Matters

### 1. **Continuous Control Rate**
Unlike Tidal/Strudel (discrete events), Phonon evaluates patterns at **sample rate** (44.1kHz).

### 2. **Unified Paradigm**
Everything is a signal. Numbers, patterns, LFOs - all the same.

### 3. **Live Modulation**
Change pattern parameters in real-time while maintaining sync.

### 4. **Compositional Power**
Stack modulations infinitely:
```phonon
sine (sine (sine 0.1 * 2 + 5) * 220 + 440)  -- Meta-modulation!
```

## Testing

All pattern parameters are tested with 3-level verification:

1. **Pattern Query**: Verifies pattern logic produces correct events
2. **Onset Detection**: Verifies audio events match pattern timing
3. **Audio Characteristics**: Verifies signal quality

**Test files**:
- `tests/test_p00_effect_patterns.rs` - Effect parameter tests (8 tests)
- `tests/test_legato_pattern.rs` - Articulation tests (4 tests)
- `tests/test_pattern_transformations.rs` - Transform tests (multiple)

**All 400+ tests passing** ✅

## Summary

✅ **Oscillators**: All frequencies
✅ **Filters**: Cutoff, Q, center
✅ **Effects**: Delay, reverb, distortion, chorus, flanger, phaser, tremolo, vibrato
✅ **Dynamics**: Compressor, limiter, amp, gain
✅ **Sample params**: Gain, pan, speed, attack, release, ar
✅ **Transforms**: Fast, slow, squeeze, legato, staccato, swing, degradeBy, shuffle

**EVERYTHING accepts patterns. No exceptions.**

---

*Last updated: 2025-11-12*
*Status: Complete - P0.0 FIXED*
