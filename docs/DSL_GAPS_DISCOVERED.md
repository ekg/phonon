# DSL Gaps Discovered Through Dual Implementation

## Critical Missing Features

### 1. **Envelope Functions** (BLOCKER for percussion)
- Native synths (kick, snare, hat) use envelopes extensively
- DSL has NO access to: `envelope`, `adsr`, `perc`, etc.
- **Impact**: Cannot replicate percussion synths
- **Needed**: `env attack decay sustain release` or similar

### 2. **Phase Control** (Limits PWM quality)
- Native superpwm uses two squares 180° out of phase
- DSL oscillators have no phase parameter
- **Workaround**: Inversion (`* -1`) approximates 180°
- **Impact**: PWM less accurate, more aliasing

### 3. **Sub-expression Reuse** (Efficiency issue)
- DSL: `square freq + square freq` creates TWO independent oscillators
- Native: Can reuse same oscillator reference
- **Impact**: Redundant computation, phase drift
- **Needed**: Let bindings or bus assignments within functions

### 4. **Pattern-Controlled Synth Triggering** (Major gap)
- **Current**: Synths work as continuous drones only
- **Missing**: `superkick "60 ~ 65 ~"` (pattern of kicks at different pitches)
- **Missing**: Rhythm patterns triggering synth+envelope combos
- **Impact**: Synths can't be used musically with patterns!

## What Works

1. ✅ Continuous drone synths (supersaw, superpwm as drones)
2. ✅ Sample playback with patterns (`s "bd sn"`)
3. ✅ User-defined functions (basic)
4. ✅ Arithmetic and signal routing

## Comparison Results

| Synth | DSL Possible? | Quality | Notes |
|-------|--------------|---------|-------|
| supersaw | YES | Excellent (0.4 dB diff) | 7-voice detuned saw |
| superpwm | PARTIAL | Good | Missing phase control |
| superchip | PARTIAL | TBD | Needs vibrato (LFO mod) |
| superfm | NO | - | Needs complex FM routing |
| superkick | NO | - | Requires envelopes |
| supersnare | NO | - | Requires envelopes + noise shaping |
| superhat | NO | - | Requires envelopes + filtered noise |

## Priority Recommendations

1. **Add envelope functions** - Enables all percussion
2. **Pattern → Synth triggering** - Makes synths usable with rhythms
3. **Phase control** - Improves PWM and other phase-sensitive algorithms
4. **Let bindings / sub-expressions** - Efficiency and clarity

## Current Status

- **Function definitions**: ✅ Working
- **Continuous synths**: ✅ supersaw replicated
- **Pattern triggering**: ❌ Not discovered in examples
- **Envelope support**: ❌ Completely missing from DSL
