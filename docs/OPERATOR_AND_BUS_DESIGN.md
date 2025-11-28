# Phonon Operator and Bus Design

## Overview

This document describes the unified design for operators and buses in Phonon. The goal is a clean, composable system where:

1. **Pattern operators** follow Tidal semantics (structure-aware)
2. **Signal operators** perform sample-by-sample audio-rate math
3. **Buses** are first-class functions that can generate, transform, or combine signals/patterns

---

## Part 1: Pattern Operators (Tidal-Compatible)

Pattern operators work on discrete events with timing structure. They follow Tidal's semantics exactly.

### Structure Operators

The `|` indicates which side provides the rhythmic structure (event timing):

```phonon
-- LEFT structure (|op): events occur at left pattern's times
"c4 e4 g4" |+ "12"        -- 3 events: c4+12, e4+12, g4+12
"c4 e4 g4" |- "5"         -- 3 events at left's timing
"c4 e4 g4" |* "2"
"c4 e4 g4" |/ "2"

-- RIGHT structure (op|): events occur at right pattern's times
"c4" +| "0 3 7"           -- 3 events: c4+0, c4+3, c4+7
"c4" -| "0 3 7"
"c4" *| "1 2"
"c4" /| "1 2"

-- BOTH structure (bare op): union of events from both patterns
"c4 e4" + "g4"            -- events from BOTH patterns combined
"100 200" * "1 2 3"       -- all event combinations
```

### Union Operators (Control Values)

For combining control patterns like gain, pan, speed:

```phonon
"0.5 0.8" |> "1.0"        -- left structure, sample right values
"0.5" <| "0.8 1.0 0.6"    -- right structure, sample left values
```

### Usage in Chains

Pattern operators work naturally with the chain operator:

```phonon
out $ s "bd" # note "c3'maj" + "0 3 7"    -- chord + offset
out $ s "bd" # note "c4" |+ "0 12 24"     -- octave spread
out $ s "bd" # gain "1" *| "0.5 0.8 1"    -- gain pattern
```

---

## Part 2: Signal Operators (Audio-Rate)

Signal operators perform sample-by-sample arithmetic on continuous audio signals. They use the `~` prefix to distinguish from pattern operators.

### Infix Form

```phonon
~mix $ ~synth1 ~+ ~synth2       -- add two signals
~mod $ ~carrier ~* ~modulator   -- AM synthesis (ring mod)
~diff $ ~sig1 ~- ~sig2          -- difference
~scaled $ ~sig ~/ 2             -- halve amplitude
```

### Prefix Form (Function Call)

The infix operators are syntactic sugar for underlying functions:

```phonon
-- These are equivalent:
~mix $ ~synth1 ~+ ~synth2
~mix $ ~add ~synth1 ~synth2

-- All signal operators have function forms:
~add a b      -- addition
~sub a b      -- subtraction
~mul a b      -- multiplication
~div a b      -- division
```

### Reserved Names

The following are reserved and cannot be used as user bus names:

- `~add`, `~sub`, `~mul`, `~div` (signal arithmetic)
- `~+`, `~-`, `~*`, `~/` (infix forms)

---

## Part 3: Buses as Functions

Buses in Phonon are first-class. Their behavior depends on how they're defined.

### Generator Buses (No Parameters)

A bus with no parameters is a signal/pattern generator:

```phonon
~lfo $ sine 0.5              -- generates LFO signal
~drums $ s "bd sn hh cp"     -- generates drum pattern
~chord $ note "c4'maj"       -- generates chord pattern

-- Usage: reference by name
out $ ~drums * 0.3
out $ saw 110 # lpf (~lfo * 1000 + 500) 0.8
```

### Transformer Buses (Effect Chains)

A bus defining an effect chain can be applied via `#`:

```phonon
~fx $ delay 0.25 0.6 # reverb 0.3
~wobble $ lpf "500 1000 2000" 0.8

-- Usage: apply to signals via chain operator
out $ ~drums # ~fx
out $ saw 110 # ~wobble
out $ ~synth # ~fx           -- same effect on different source
```

### Function Buses (Explicit Parameters)

Parameters before `$` create a callable function:

```phonon
~mix a b $ a ~* 0.7 ~+ b ~* 0.3
~sidechained kick src $ src ~* (1 ~- (kick # env 0.01 0.1))
~harmony base interval $ base ~+ (base # transpose interval)

-- Usage: call with arguments
out $ ~mix ~drums ~synth
out $ ~sidechained ~kick ~bass
out $ ~harmony ~lead 12       -- octave up harmony
```

### Higher-Order Buses (Bus as Parameter)

Buses can take other buses as parameters:

```phonon
~stereoize f $ f # juxBy 0.5 rev
~doubled f $ f ~+ (f # delaySamples 100)
~withVerb f $ f # reverb 0.3

-- Usage: pass buses as arguments
~wideDrums $ ~stereoize ~drums
~fatBass $ ~doubled ~bass
~wetSynth $ ~withVerb ~synth
```

### The Unifying Principle

The presence of parameters before `$` determines bus type:

| Definition | Type | Usage |
|------------|------|-------|
| `~name $ expr` | Generator | `~name` (reference) |
| `~name $ effect chain` | Transformer | `signal # ~name` |
| `~name a b $ expr` | Function | `~name arg1 arg2` |

---

## Part 4: Type Flow

### Pattern vs Signal Context

The system infers whether operations are pattern-level or signal-level:

```phonon
-- Pattern context (quoted strings, pattern functions)
"100 200" + "50"              -- pattern addition
s "bd sn" # note "c4" + "12"  -- pattern note offset

-- Signal context (bus references, oscillators)
~osc1 ~+ ~osc2                -- signal addition
sine 440 ~* sine 2            -- AM synthesis
```

### Mixing Patterns and Signals

Patterns can control signal parameters:

```phonon
~bass $ saw "55 82.5 110"     -- pattern controls frequency
~filt $ lpf "500 1000" 0.8    -- pattern controls cutoff

-- Signal operations on pattern-controlled sources
out $ ~bass ~* ~envelope
```

---

## Part 5: Complete Examples

### Example 1: Layered Synthesis

```phonon
-- Oscillators
~sub $ saw 55
~mid $ saw 110 # detune 0.1
~top $ square 220

-- Mix with signal operators
~stack $ ~sub ~* 0.5 ~+ ~mid ~* 0.3 ~+ ~top ~* 0.2

-- Filter with pattern-controlled cutoff
~lfo $ sine 0.25
out $ ~stack # lpf (~lfo * 2000 + 500) 0.7
```

### Example 2: Drum Processing

```phonon
-- Sources
~kick $ s "bd:3"
~snare $ s "~ sn ~ sn"
~hats $ s "hh*8"

-- Effect chains as transformers
~punch $ hpf 60 1 # compress 4 0.01 0.1
~room $ reverb 0.2 # lpf 4000 0.5

-- Mixing function
~drumMix k s h $ k ~* 1.0 ~+ s ~* 0.8 ~+ h ~* 0.4

-- Final mix
~drums $ ~drumMix (~kick # ~punch) (~snare # ~room) ~hats
out $ ~drums # limiter 0.9
```

### Example 3: Melodic Pattern with Harmony

```phonon
-- Base melody
~melody $ note "c4 e4 g4 b4" # sound "piano"

-- Harmony function
~addHarmony base interval $ base ~+ (base # transpose interval)

-- Chord voicing function
~voicing root $ root |+ "0 4 7"    -- major triad structure

-- Build arrangement
~lead $ ~melody # ~voicing
~full $ ~addHarmony ~lead 12       -- add octave

out $ ~full # reverb 0.3
```

### Example 4: Sidechain Compression

```phonon
-- Sidechain function
~sidechain trigger src depth $
    src ~* (1 ~- (trigger # env 0.001 0.15) ~* depth)

-- Sources
~kick $ s "bd ~ ~ bd ~ ~ bd ~"
~pad $ saw "c3 e3 g3" # lpf 800 0.6

-- Apply sidechain
out $ ~sidechain ~kick ~pad 0.8
```

---

## Implementation Phases

### Phase 1: Pattern Operators ✅ COMPLETE
- [x] Left structure operators: `|+`, `|-`, `|*`, `|/`
- [x] Right structure operators: `+|`, `-|`, `*|`, `/|`
- [x] Union operators: `|>`, `<|`
- [x] Both structure operators: bare `+`, `-`, `*`, `/` use union semantics
  - Implemented `add_both`, `sub_both`, `mul_both`, `div_both` in Pattern
  - Compiler routes bare operators to both-structure methods when operands are patterns
  - Tests: `test_both_structure_operators.rs` (11 tests)

### Phase 2: Signal Operators ✅ COMPLETE
- [x] Add `~+`, `~-`, `~*`, `~/` infix operators to parser
  - Added `SignalAdd`, `SignalSub`, `SignalMul`, `SignalDiv` to BinOp enum
  - Parser handles signal operators in additive/multiplicative expression parsing
- [x] Add `~add`, `~sub`, `~mul`, `~div` function forms
  - Implemented `parse_signal_function_call` for prefix notation
  - Supports syntax like `~add (sine 440) (sine 441)`
- [x] Reserve these names from user bus definitions
  - Added `RESERVED_SIGNAL_NAMES` in compiler
  - Compiler rejects `~add`, `~sub`, `~mul`, `~div` as bus names
- [x] Route to sample-by-sample signal arithmetic in compiler
  - Signal operators bypass pattern-level combination
  - Direct sample-by-sample `SignalExpr::Add/Subtract/Multiply/Divide`
  - Tests: `test_signal_operators.rs` (14 tests)

### Phase 3: Buses as Functions ✅ COMPLETE
- [x] Transformer buses (effect chains via `#`)
  - Already worked: `~fx $ lpf 1000 0.8` then `saw 110 # ~fx`
- [x] Function buses: `~name a b $ expr` syntax
  - Parameters before `$` create callable functions
  - Example: `~mix a b $ a ~* 0.5 ~+ b ~* 0.5`
- [x] Bus calling with arguments
  - `parse_bus_call_expr` handles `~name arg1 arg2` syntax
  - Compiler stores function definitions and instantiates on call
- [x] Higher-order bus support
  - Buses can take other buses as parameters
  - Example: `~doubled f $ f ~+ f` then `~doubled ~osc`
  - Parameter substitution via compile-time bindings
  - Tests: `test_bus_as_function.rs` (13 tests)

### Phase 4: Type Inference ✅ COMPLETE
- [x] Automatic pattern vs signal context detection
  - Quoted strings (`"..."`) are patterns
  - Oscillators (`sine`, `saw`, etc.) are signals
  - Bus references adapt based on bus contents
- [x] Clean error messages for type mismatches
  - Undefined bus errors include bus name
  - Wrong arity errors show expected/actual count
- [x] Optimization of known-constant patterns to signals
  - Single-value patterns like `"440"` work equivalently to `440`
  - Tests: `test_type_inference.rs` (12 tests)

---

## Appendix: Operator Precedence

From lowest to highest:

1. `$` (function application / transform)
2. `#` (chain / effect application)
3. `|>`, `<|` (union operators)
4. `+`, `-`, `|+`, `+|`, `|-`, `-|`, `~+`, `~-` (additive)
5. `*`, `/`, `|*`, `*|`, `|/`, `/|`, `~*`, `~/` (multiplicative)
6. Unary `-` (negation)
7. Function application (juxtaposition)

---

## Design Rationale

### Why `~` for Signal Operators?

1. **Visual consistency**: `~` already means "bus/signal" in `~busName`
2. **Clear distinction**: `+` is pattern, `~+` is signal - no ambiguity
3. **Extensibility**: Any function can have a signal-rate form: `~fn`

### Why Buses as Functions?

1. **Reusability**: Define once, apply many times
2. **Composition**: Build complex processing from simple parts
3. **Abstraction**: Hide implementation details behind clean interfaces

### Why Tidal-Compatible Pattern Operators?

1. **Familiarity**: Existing Tidal users feel at home
2. **Proven design**: 10+ years of live coding refinement
3. **Expressiveness**: Structure operators enable powerful pattern manipulation

---

*This document serves as the north star for Phonon's operator and bus system implementation.*
