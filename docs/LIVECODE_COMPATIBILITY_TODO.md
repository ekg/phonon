# Livecode Compatibility TODO

**CRITICAL: See docs/CRITICAL_BUGS.md for blocking issues that must be fixed FIRST**

Last updated: 2025-11-10

## 🔴 BLOCKING ISSUES (Fix Before Anything Else)

**See docs/CRITICAL_BUGS.md for details**

These issues make Phonon unusable in production:
1. **P0.0**: ALL parameters must accept patterns (not bare numbers) - ARCHITECTURAL
2. **P0.1**: Delay not bus-specific/chainable - BROKEN
3. **P0.2**: stack multiplies volume instead of mixing - BROKEN
4. **P0.3**: Output volumes affect each other - BROKEN
5. **P0.4**: Multi-threading broken, poor performance - BROKEN

**DO NOT ADD NEW FEATURES UNTIL THESE ARE FIXED**

---

## ✅ COMPLETED (P0/P1 - Before Critical Issues Discovered)

Features that work (but may need pattern parameter support):
- ✅ jux/juxBy - Stereo panning with transforms
- ✅ loopAt patterns - Pattern-controlled durations
- ✅ striate/slice - Sample slicing with begin/end context
- ✅ legato - ADSR envelope with auto-release
- ✅ Transform chains - `jux (fast 2 $ rev)` syntax
- ✅ struct - Apply structure/rhythm (284 uses in livecode)

## 🎯 P2 - HIGH PRIORITY (By Usage Frequency)

### 1. struct (284 uses) ❌ NOT IMPLEMENTED
**What it does**: Apply structure/rhythm from one pattern to another

**Examples from livecode**:
```tidal
struct "t(3,7)" $ s "808mt(<12 12 13>,14,11)"
-- Uses euclidean pattern "t(3,7)" as the trigger structure
-- Applies it to the sample pattern

struct "t t ~ t" $ note "0 2 4"
-- Four-beat structure with silence on beat 3
-- Applies to note pattern
```

**Implementation notes**:
- Takes a boolean/trigger pattern and a value pattern
- For each trigger in structure pattern, pull value from value pattern
- Essentially: `structurePattern.withValue(valuePattern)`

**Status**: ❌ Not in Transform enum, needs implementation

---

### 2. stut (132 uses) ⚠️ PARTIAL
**What it does**: Stutter/echo with multiple repeats and feedback

**Examples from livecode**:
```tidal
stut 2 0.7 (3/16) $ note "0 2 4"
-- 2 repeats, 0.7 feedback (each repeat quieter), 3/16 time offset

stut 3 0.7 (3/8) $ compress (1/4, 3/4) $ every 16 (* note "0 7 -12 -5")
-- 3 repeats with feedback
```

**Current status**:
- ⚠️ `stutter` exists but only repeats once (no feedback/decay)
- Need multi-repeat version with feedback parameter

**Implementation notes**:
- `stut n feedback time pattern` creates n copies
- Each copy: offset by `time * i`, gain multiplied by `feedback ^ i`
- In Phonon: likely via pattern operation that creates multiple offset events

**Status**: ⚠️ Single-repeat `stutter` exists, need proper multi-repeat `stut`

---

### 3. hurry (37 uses) ❌ NOT IMPLEMENTED
**What it does**: Speed up both rhythm AND pitch (combines fast + speed)

**Examples from livecode**:
```tidal
hurry 2  -- Equivalent to: fast 2 # speed 2
hurry 0.5  -- Equivalent to: slow 2 # speed 0.5

foldEvery [2,4] (hurry 0.5) $ fast 2 $ n "3796" # m
```

**Implementation notes**:
- `hurry n` = `fast n` with `speed n` applied
- Simple transform that combines two existing operations
- Should be easy to implement

**Status**: ❌ Not in Transform enum, needs implementation

---

### 4. off (35 uses) ⚠️ NEEDS FUNCTION VERSION
**What it does**: Offset pattern by time and apply transform

**Examples from livecode**:
```tidal
off 0.125 (# speed (4/3)) $ n "2129*4" # m
-- Copy pattern 0.125 cycles later with speed transform

off "<0.125 0.25 0.125 0.5>" (+ note "-5")
-- Offset varies per cycle, transpose down 5 semitones

sometimes (off (cat [(3/16), (1/16), (4/17), (3/16)]) (# speed "<0.5 0.666 0.5 0.333>"))
```

**Current status**:
- ⚠️ `Offset(Box<Expr>)` exists as simple time shift
- Need `Off` that takes time AND transform: `off time transform pattern`

**Implementation notes**:
- Creates two patterns: original + (late time $ transform pattern)
- Stack them together
- In Phonon: `Pattern::new(|state| stack(original, late(time, transform(pattern))))`

**Status**: ⚠️ `offset` exists, need `Off { time, transform }` version

---

### 5. foldEvery (7 uses) ❌ NOT IMPLEMENTED
**What it does**: Like `every` but for multiple cycle numbers

**Examples from livecode**:
```tidal
foldEvery [2,3,4] (fast 2)
-- Apply fast 2 on cycles: 2, 3, 4, 6, 8, 9, 10, 12, ...

foldEvery [2,8] (fast 4) $ s "hc" # n 2
```

**Implementation notes**:
- `foldEvery [n1, n2, n3] transform pattern`
- Apply transform on cycle if cycle_num divisible by ANY of [n1, n2, n3]
- Like: `every n1 transform . every n2 transform . every n3 transform`

**Status**: ❌ Not in Transform enum, needs implementation

---

## P3 - MEDIUM PRIORITY

### 6. compress (6 uses) ⚠️ DEFINED, NEEDS TESTING
```tidal
compress (1/4, 3/4) $ note "0 2 4"
-- Compress pattern to window from 1/4 to 3/4 of cycle
```
**Status**: ⚠️ In Transform enum, needs testing

### 7. sew (5 uses) ❌ NOT IMPLEMENTED
```tidal
sew "t(3,8)" patternA patternB
-- Switch between two patterns based on boolean pattern
```
**Status**: ❌ Not implemented

### 8. sustain - ❌ NOT IMPLEMENTED
Sample parameter for controlling sample length without legato.
**Status**: ❌ Not implemented as parameter

### 9. splice - ❌ NOT IMPLEMENTED
Like slice but adjusts speed to fit original duration.
**Status**: ❌ Not implemented

---

## Implementation Order (Recommended)

1. **struct** (284 uses) - Highest impact, fundamental pattern operation
2. **hurry** (37 uses) - Easy win, combines fast + speed
3. **stut** (132 uses) - Echo/delay effect, needs multi-repeat with feedback
4. **off** (35 uses) - Offset + transform, useful for harmonies/delays
5. **foldEvery** (7 uses) - Extends `every` for multiple cycles

## Test Strategy

For each implementation:
1. **Level 1**: Pattern query verification (event count, structure)
2. **Level 2**: Onset detection (audio event timing)
3. **Level 3**: Audio characteristics (RMS, signal quality)
4. **Integration**: Real livecode examples

## Notes

- Most of these are used in combination (e.g., `foldEvery [2,4] (hurry 0.5)`)
- struct is the most used and likely most impactful
- hurry is the easiest to implement (just fast + speed)
- stut already has foundation (stutter), needs enhancement
- off needs new variant that takes transform parameter
